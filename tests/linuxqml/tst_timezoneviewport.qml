import QtQuick
import QtTest
import QtPositioning
import "../../qml"

Item {
    id: host
    width: 1100
    height: 900
    property var selections: []
    Component {
        id: mapComponent
        KdeTimezoneMap {
            zone: ""
            center: [40, -105]
            preferredZoom: 2
            onZoneSelected: id => host.selections = host.selections.concat(id)
        }
    }
    Component {
        id: clippingFixture
        Rectangle {
            id: fixture
            width: 1100; height: 700
            color: "#dddddd"
            property real uiScale: 1
            property alias adapter: scaledMap
            property alias mapFrame: mapFrame
            Item {
                id: mapFrame
                x: 200; y: 150; width: 675; height: 255
                clip: true
                KdeTimezoneMap {
                    id: scaledMap
                    width: mapFrame.width / fixture.uiScale
                    height: mapFrame.height / fixture.uiScale
                    scale: fixture.uiScale
                    transformOrigin: Item.TopLeft
                    center: [40,-105]
                    preferredZoom: 2
                    initialBounds: QtPositioning.rectangle(QtPositioning.coordinate(50,-125),QtPositioning.coordinate(24,-66))
                }
            }
        }
    }
    Component {
        id: chinaPickerComponent
        TimezonePicker {
            mapEnabled: true
            region: "CN"
            selectedCode: ""
            zones: [
                {code:"Asia/Shanghai",label:"Shanghai",suggested:true,center:[31.2333,121.4667]},
                {code:"Asia/Urumqi",label:"Urumqi",suggested:true,center:[43.8,87.5833]}
            ]
            onZoneSelected: id => {
                host.selections = host.selections.concat(id)
                selectedCode = id
            }
        }
    }
    SignalSpy { id: modelChanges; signalName: "visiblePolygonsChanged" }
    SignalSpy { id: viewportChanges; signalName: "renderBoundsChanged" }
    TestCase {
        name: "TimezoneViewport"
        when: windowShown
        function init() { failOnWarning(/.?/); host.selections = [] }
        function ready(subject) {
            verify(subject)
            tryVerify(() => subject.mapItem.mapReady && !subject.fitting,5000)
            wait(200)
            if (subject.surface.visiblePolygons.length < 2) console.log("EMPTY_VIEWPORT " + JSON.stringify({
                size:[subject.mapItem.width,subject.mapItem.height],zoom:subject.mapItem.zoomLevel,
                center:subject.mapItem.center,bounds:subject.surface.renderBounds,polygons:subject.surface.polygons.length,
                nw:subject.mapItem.toCoordinate(Qt.point(0,0),false),
                se:subject.mapItem.toCoordinate(Qt.point(subject.mapItem.width,subject.mapItem.height),false),
                first:subject.surface.polygons[0]?.bounds
            }))
            tryVerify(() => subject.surface.visiblePolygons.length > 1,5000)
            renderFrame(subject)
        }
        function renderFrame(subject) {
            // Offscreen tests do not guarantee a paint after camera updates.
            // Test input against the very first rendered frame, not zero-size
            // delegates that are still waiting for Qt's geometry polish.
            verify(grabImage(subject).width > 0)
        }
        function hit(subject, latitude, longitude, zone) {
            const map = subject.mapItem
            const coordinate = QtPositioning.coordinate(latitude,longitude)
            const point = map.fromCoordinate(coordinate,false)
            if (point.x <= 2 || point.x >= map.width - 2 || point.y <= 2 || point.y >= map.height - 2) return false
            const polygons = Array.from(map.mapItems).filter(item => item.tzid === zone)
            verify(polygons.length > 0,zone + " has no rendered area")
            verify(polygons.every(item => item.autoFadeIn === false))
            const selectable = () => polygons.some(item => item.geoShape.contains(coordinate)
                && item.contains(map.mapToItem(item,point.x,point.y)))
            if (!selectable()) {
                console.log("HIT_FAILURE " + JSON.stringify({zone,point,zoom:map.zoomLevel,center:map.center,
                    size:[map.width,map.height],polygons:polygons.map(item => ({
                        size:[item.width,item.height],position:[item.x,item.y],visible:item.visible,
                        geo:item.geoShape.contains(coordinate),local:map.mapToItem(item,point.x,point.y)
                    }))}))
            }
            tryVerify(selectable,1000,zone + " is visible but not selectable")
            const before = host.selections.length
            mouseClick(map,point.x,point.y)
            tryCompare(host,"selections",host.selections.slice(0,before).concat(zone))
            return true
        }
        function test_low_zoom_areas_stay_selectable_data() {
            return [{tag:"wide",width:1067,height:252},{tag:"compact",width:675,height:255},
                {tag:"portrait",width:275,height:720}]
        }
        function test_low_zoom_areas_stay_selectable(data) {
            const subject = createTemporaryObject(mapComponent,host,{width:data.width,height:data.height})
            ready(subject)
            const minimum = subject.mapItem.minimumZoomLevel
            for (const zoom of [minimum,minimum + 0.1,minimum + 0.5,3.5]) {
                subject.mapItem.zoomLevel = zoom
                subject.mapItem.center = QtPositioning.coordinate(40,-105)
                wait(200)
                renderFrame(subject)
                verify(subject.surface.renderBounds.isValid)
                let checked = 0
                // Use Kansas City, well inside Central time, rather than a
                // lakeshore pixel that can round outside Chicago when tiny.
                for (const row of [[40.7142,-74.0064,"America/New_York"],[39.0997,-94.5786,"America/Chicago"],
                        [39.7392,-104.9903,"America/Denver"],[34.0522,-118.2437,"America/Los_Angeles"]])
                    if (hit(subject,...row)) checked++
                verify(checked >= 2,"Not enough U.S. regions were visible at zoom " + subject.mapItem.zoomLevel)
            }
        }
        function test_tiny_island_pointer_rounding_data() {
            return [{tag:"compact",width:675,height:255},{tag:"portrait",width:275,height:720}]
        }
        function test_tiny_island_pointer_rounding(data) {
            const subject = createTemporaryObject(mapComponent,host,{
                width:data.width,height:data.height,center:[-40,-80]
            })
            ready(subject)
            subject.mapItem.zoomLevel = subject.mapItem.minimumZoomLevel
            wait(200)
            renderFrame(subject)
            const map = subject.mapItem
            const point = map.fromCoordinate(QtPositioning.coordinate(-27.1885,-109.3017),false)
            const snapped = Qt.point(Math.round(point.x),Math.round(point.y))
            const coordinate = map.toCoordinate(snapped,false)
            compare(subject.surface.timezoneAt(coordinate),"Pacific/Easter")
            // The tolerance is one input pixel, not an enlarged arbitrary
            // ocean region. Nearby open water must still select nothing.
            compare(subject.surface.timezoneAt(map.toCoordinate(Qt.point(snapped.x+8,snapped.y),false)),"")
            verify(hit(subject,-27.1885,-109.3017,"Pacific/Easter"))
        }
        function test_dateline_view_keeps_both_sides_selectable() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            subject.mapItem.zoomLevel = subject.mapItem.minimumZoomLevel + 0.2
            subject.mapItem.center = QtPositioning.coordinate(40,175)
            wait(200)
            renderFrame(subject)
            verify(subject.surface.renderBounds.isValid)
            verify(subject.surface.renderBounds.topLeft.longitude > subject.surface.renderBounds.bottomRight.longitude)
            verify(hit(subject,35.6762,139.6503,"Asia/Tokyo"))
            verify(hit(subject,34.0522,-118.2437,"America/Los_Angeles"))
        }
        function test_world_view_includes_other_continents_without_zooming() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            subject.mapItem.zoomLevel = subject.mapItem.minimumZoomLevel
            wait(200)
            renderFrame(subject)
            compare(subject.surface.renderBounds.width,360)
            verify(hit(subject,48.8566,2.3522,"Europe/Paris"))
            verify(hit(subject,35.6762,139.6503,"Asia/Tokyo"))
            verify(hit(subject,4.711,-74.0721,"America/Bogota"))
        }
        function test_country_areas_at_wide_zoom_data() {
            const rows = []
            for (const size of [[1067,252,"wide"],[675,255,"compact"],[275,720,"portrait"]]) {
                rows.push({tag:"Russia-"+size[2],width:size[0],height:size[1],center:[58,100],points:[
                    [55.7558,37.6178,"Europe/Moscow"],
                    [56.8389,60.6057,"Asia/Yekaterinburg"],
                    [56.0153,92.8932,"Asia/Krasnoyarsk"],
                    [62.0355,129.6755,"Asia/Yakutsk"],
                    [43.1155,131.8855,"Asia/Vladivostok"],
                    [53.0167,158.65,"Asia/Kamchatka"]
                ]})
                rows.push({tag:"China-"+size[2],width:size[0],height:size[1],center:[37.5,104.5],points:[
                    [31.2333,121.4667,"Asia/Shanghai"],
                    [43.8,87.5833,"Asia/Urumqi"]
                ]})
                rows.push({tag:"Australia-"+size[2],width:size[0],height:size[1],center:[-27,134],points:[
                    [-31.95,115.85,"Australia/Perth"],
                    [-12.4667,130.8333,"Australia/Darwin"],
                    [-34.9167,138.5833,"Australia/Adelaide"],
                    [-27.4667,153.0333,"Australia/Brisbane"],
                    [-33.8667,151.2167,"Australia/Sydney"]
                ]})
                rows.push({tag:"South-Africa-"+size[2],width:size[0],height:size[1],center:[-29,25],points:[
                    [-26.25,28,"Africa/Johannesburg"],
                    [-33.9249,18.4241,"Africa/Johannesburg"]
                ]})
                rows.push({tag:"Brazil-"+size[2],width:size[0],height:size[1],center:[-12,-52],points:[
                    [-23.5333,-46.6167,"America/Sao_Paulo"],
                    [-3.1333,-60.0167,"America/Manaus"],
                    [-9.9667,-67.8,"America/Rio_Branco"],
                    [-1.45,-48.4833,"America/Belem"]
                ]})
                rows.push({tag:"Canada-"+size[2],width:size[0],height:size[1],center:[58,-104],points:[
                    [43.65,-79.3833,"America/Toronto"],
                    [49.8833,-97.15,"America/Winnipeg"],
                    [53.55,-113.4667,"America/Edmonton"],
                    [49.2667,-123.1167,"America/Vancouver"]
                ]})
                rows.push({tag:"India-"+size[2],width:size[0],height:size[1],center:[22,82],points:[
                    [28.6139,77.209,"Asia/Kolkata"],
                    [12.9716,77.5946,"Asia/Kolkata"]
                ]})
                rows.push({tag:"Indonesia-"+size[2],width:size[0],height:size[1],center:[-3,120],points:[
                    [-6.9,107.6,"Asia/Jakarta"],
                    [-0.5,111,"Asia/Pontianak"],
                    [-3.5,120,"Asia/Makassar"],
                    [-4.08,138.95,"Asia/Jayapura"]
                ]})
                rows.push({tag:"New-Zealand-"+size[2],width:size[0],height:size[1],center:[-40,179],points:[
                    [-37.78,175.28,"Pacific/Auckland"],
                    [-43.95,-176.55,"Pacific/Chatham"]
                ]})
                rows.push({tag:"Chile-"+size[2],width:size[0],height:size[1],center:[-40,-80],points:[
                    [-33.45,-70.6667,"America/Santiago"],
                    [-53.15,-70.9167,"America/Punta_Arenas"],
                    [-27.1885,-109.3017,"Pacific/Easter"]
                ]})
                rows.push({tag:"Nepal-"+size[2],width:size[0],height:size[1],center:[28,84],points:[
                    [27.7167,85.3167,"Asia/Kathmandu"],
                    [28.2,83.98,"Asia/Kathmandu"]
                ]})
            }
            return rows
        }
        function test_country_areas_at_wide_zoom(data) {
            const subject = createTemporaryObject(mapComponent,host,{
                width:data.width,height:data.height,center:data.center
            })
            ready(subject)
            for (const zoom of [subject.mapItem.minimumZoomLevel,subject.mapItem.minimumZoomLevel+0.25,2]) {
                subject.mapItem.zoomLevel = zoom
                subject.mapItem.center = QtPositioning.coordinate(...data.center)
                wait(200)
                renderFrame(subject)
                let checked = 0
                for (const point of data.points) if (hit(subject,...point)) checked++
                verify(checked >= 2,data.tag+" did not expose two selectable areas at zoom "+subject.mapItem.zoomLevel)
            }
        }
        function test_china_picker_map_selection_data() {
            return [{tag:"wide",width:1067,height:650},{tag:"compact",width:675,height:650},
                {tag:"portrait",width:275,height:900}]
        }
        function test_china_picker_map_selection(data) {
            const picker = createTemporaryObject(chinaPickerComponent,host,{width:data.width,height:data.height})
            verify(picker)
            tryVerify(() => picker.mapItem !== null,5000)
            ready(picker.mapItem)
            const point = picker.mapItem.mapItem.fromCoordinate(QtPositioning.coordinate(43.8,87.5833),false)
            mouseMove(picker.mapItem.mapItem,point.x,point.y)
            tryCompare(picker.mapItem.surface,"hoveredTimeZone","Asia/Urumqi")
            verify(hit(picker.mapItem,43.8,87.5833,"Asia/Urumqi"))
            tryCompare(picker,"selectedCode","Asia/Urumqi")
            tryVerify(() => !picker.mapItem.fitting,5000)
            compare(picker.mapItem.highlightedZone,"Asia/Urumqi")
        }
        function test_new_zealand_initial_picker_fits_both_sides_of_dateline_data() {
            return [{tag:"wide",width:1067,height:650},{tag:"compact",width:675,height:650},
                {tag:"portrait",width:275,height:900}]
        }
        function test_new_zealand_initial_picker_fits_both_sides_of_dateline(data) {
            const picker = createTemporaryObject(chinaPickerComponent,host,{
                width:data.width,height:data.height,region:"NZ",zones:[
                    {code:"Pacific/Auckland",label:"Auckland",suggested:true,center:[-36.8667,174.7667]},
                    {code:"Pacific/Chatham",label:"Chatham",suggested:true,center:[-43.95,-176.55]}
                ]
            })
            verify(picker)
            tryVerify(() => picker.mapItem !== null,5000)
            const subject = picker.mapItem
            ready(subject)
            verify(Math.abs(subject.mapItem.center.longitude) > 170)
            verify(subject.areaBounds && subject.areaBounds.isValid)
            verify(subject.mapItem.zoomLevel > subject.mapItem.minimumZoomLevel+0.5)
            const islands = subject.surface.polygons.filter(row =>
                ["Pacific/Auckland","Pacific/Chatham"].includes(row.tzid))
            verify(islands.length >= 2)
            for (const row of islands) for (const coordinate of row.coordinates) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1,row.tzid+" east/west border")
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1,row.tzid+" north/south border")
            }
            verify(!picker.canConfirm)
            for (const point of [[-37.78,175.28,"Pacific/Auckland"],[-43.95,-176.55,"Pacific/Chatham"]]) {
                picker.selectedCode = ""
                ready(subject)
                verify(hit(subject,...point))
                tryCompare(picker,"selectedCode",point[2])
                tryVerify(() => !subject.fitting,5000)
                compare(subject.highlightedZone,point[2])
                verify(picker.canConfirm)
            }
        }
        function test_indonesia_and_chile_initial_picker_data() {
            const rows = []
            for (const size of [[1067,650,"wide"],[675,650,"compact"],[275,900,"portrait"]]) {
                rows.push({tag:"Indonesia-"+size[2],width:size[0],height:size[1],region:"ID",zones:[
                    {code:"Asia/Jakarta",center:[-6.1667,106.8]},
                    {code:"Asia/Pontianak",center:[-0.0333,109.3333]},
                    {code:"Asia/Makassar",center:[-5.1167,119.4]},
                    {code:"Asia/Jayapura",center:[-2.5333,140.7]}
                ],points:[[-6.9,107.6,"Asia/Jakarta"],[-0.5,111,"Asia/Pontianak"],
                    [-3.5,120,"Asia/Makassar"],[-4.08,138.95,"Asia/Jayapura"]]})
                rows.push({tag:"Chile-"+size[2],width:size[0],height:size[1],region:"CL",zones:[
                    {code:"America/Santiago",center:[-33.45,-70.6667]},
                    {code:"America/Coyhaique",center:[-45.5667,-72.0667]},
                    {code:"America/Punta_Arenas",center:[-53.15,-70.9167]},
                    {code:"Pacific/Easter",center:[-27.15,-109.4333]}
                ],points:[[-33.45,-70.6667,"America/Santiago"],[-53.15,-70.9167,"America/Punta_Arenas"],
                    [-27.1885,-109.3017,"Pacific/Easter"]]})
            }
            return rows
        }
        function test_indonesia_and_chile_initial_picker(data) {
            const picker = createTemporaryObject(chinaPickerComponent,host,{
                width:data.width,height:data.height,region:data.region,
                zones:data.zones.map(zone => Object.assign({label:zone.code,suggested:true},zone))
            })
            verify(picker)
            tryVerify(() => picker.mapItem !== null,5000)
            const subject = picker.mapItem
            ready(subject)
            verify(subject.areaBounds && subject.areaBounds.isValid)
            verify(subject.mapItem.zoomLevel > subject.mapItem.minimumZoomLevel+0.2)
            verify(!picker.canConfirm)
            for (const coordinate of [subject.areaBounds.topLeft,subject.areaBounds.topRight,
                    subject.areaBounds.bottomLeft,subject.areaBounds.bottomRight]) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1)
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1)
            }
            for (const point of data.points) {
                picker.selectedCode = ""
                ready(subject)
                verify(hit(subject,...point))
                tryCompare(picker,"selectedCode",point[2])
                tryVerify(() => !subject.fitting,5000)
                compare(subject.highlightedZone,point[2])
                verify(picker.canConfirm)
            }
            // Newer tzdata can precede KDE's geographic outlines. The list
            // must still select and confirm Coyhaique without a fake polygon.
            if (data.region === "CL") {
                picker.searchText = "Coyhaique"
                compare(picker.confirmationCode,"America/Coyhaique")
                picker.confirm()
                compare(picker.selectedCode,"America/Coyhaique")
                tryVerify(() => !subject.fitting,5000)
                compare(subject.highlightedZone,"America/Coyhaique")
            }
        }
        function test_panning_the_world_does_not_recreate_unchanged_areas() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            subject.mapItem.zoomLevel = subject.mapItem.minimumZoomLevel
            wait(200)
            renderFrame(subject)
            const items = Array.from(subject.mapItem.mapItems)
            compare(items.length,subject.surface.visiblePolygons.length)
            modelChanges.target = subject.surface
            modelChanges.clear()
            subject.mapItem.pan(5,0)
            wait(200)
            compare(modelChanges.count,0)
            const after = Array.from(subject.mapItem.mapItems)
            compare(after.length,items.length)
            verify(items.every(item => after.includes(item)))
            modelChanges.target = null
        }
        function test_scaled_map_stays_inside_frame_data() {
            return [{tag:"small",scale:600/820},{tag:"compact",scale:720/820},{tag:"large",scale:1.25}]
        }
        function test_viewport_keeps_up_during_continuous_panning() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            subject.mapItem.zoomLevel = 3.5
            wait(200)
            renderFrame(subject)
            viewportChanges.target = subject.surface
            viewportChanges.clear()
            for (let step = 0; step < 20; ++step) {
                subject.mapItem.pan(3,0)
                wait(20)
            }
            verify(viewportChanges.count >= 2,"New areas were postponed until panning stopped")
            viewportChanges.target = null
        }
        function test_scaled_map_stays_inside_frame(data) {
            const fixture = createTemporaryObject(clippingFixture,host,{uiScale:data.scale})
            ready(fixture.adapter)
            mouseMove(fixture,0,0)
            const frame = fixture.mapFrame
            const image = grabImage(fixture)
            for (let y = 0; y < fixture.height; y += 3)
                for (let x = 0; x < fixture.width; x += 3)
                    if (x < frame.x - 2 || x > frame.x + frame.width + 2 || y < frame.y - 2 || y > frame.y + frame.height + 2)
                        compare(image.pixel(x,y),fixture.color,"Map painted outside its frame at " + x + "," + y)
        }
        function test_us_mainland_initial_framing_data() {
            return [{tag:"wide",width:1067,height:252},{tag:"compact",width:675,height:255},
                {tag:"portrait",width:275,height:720}]
        }
        function test_us_mainland_initial_framing(data) {
            const bounds = QtPositioning.rectangle(QtPositioning.coordinate(50,-125),QtPositioning.coordinate(24,-66))
            const subject = createTemporaryObject(mapComponent,host,{width:data.width,height:data.height,initialBounds:bounds})
            ready(subject)
            verify(subject.mapItem.zoomLevel > subject.mapItem.minimumZoomLevel + 0.5)
            for (const coordinate of [bounds.topLeft,bounds.topRight,bounds.bottomLeft,bounds.bottomRight]) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin - 1 && point.x <= subject.mapItem.width - subject.fitMargin + 1)
                verify(point.y >= subject.fitMargin - 1 && point.y <= subject.mapItem.height - subject.fitMargin + 1)
            }
            verify(hit(subject,40.7142,-74.0064,"America/New_York"))
            verify(hit(subject,34.0522,-118.2437,"America/Los_Angeles"))
            for (const zone of ["America/Anchorage","Pacific/Honolulu"]) {
                subject.zone = zone
                tryVerify(() => subject.fittedZone === zone && !subject.fitting,5000)
                verify(subject.selectedCoordinates().length > 0)
                compare(subject.areaBounds,subject.selectedBounds())
            }
        }
        function test_australia_initial_framing_data() {
            return [{tag:"wide",width:1067,height:252},{tag:"compact",width:675,height:255},
                {tag:"portrait",width:275,height:720}]
        }
        function test_australia_initial_framing(data) {
            const bounds = QtPositioning.rectangle(QtPositioning.coordinate(-9,112),QtPositioning.coordinate(-44.5,160))
            const subject = createTemporaryObject(mapComponent,host,{width:data.width,height:data.height,initialBounds:bounds})
            ready(subject)
            verify(subject.mapItem.zoomLevel > subject.mapItem.minimumZoomLevel+0.5)
            for (const coordinate of [bounds.topLeft,bounds.topRight,bounds.bottomLeft,bounds.bottomRight]) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1)
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1)
            }
            const australian = subject.surface.polygons.filter(row => row.tzid.startsWith("Australia/")
                && row.bounds.topLeft.latitude >= -60)
            verify(australian.length > 10)
            for (const row of australian) for (const coordinate of row.coordinates) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1,row.tzid+" east/west border")
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1,row.tzid+" north/south border")
            }
            // Inland points avoid pixel rounding along a coastline at wide zoom.
            verify(hit(subject,-30.75,121.47,"Australia/Perth"))
            verify(hit(subject,-14,132.5,"Australia/Darwin"))
            verify(hit(subject,-32.25,148.6,"Australia/Sydney"))
            verify(hit(subject,-42.8,147.3,"Australia/Hobart"))
            const lordHowe = subject.mapItem.fromCoordinate(QtPositioning.coordinate(-31.55,159.0833),false)
            verify(lordHowe.x >= subject.fitMargin && lordHowe.x <= subject.mapItem.width-subject.fitMargin)
            verify(lordHowe.y >= subject.fitMargin && lordHowe.y <= subject.mapItem.height-subject.fitMargin)
            // Distant islands still fit independently after a selection.
            for (const zone of ["Australia/Lord_Howe","Antarctica/Macquarie"]) {
                subject.zone = zone
                tryVerify(() => subject.fittedZone === zone && !subject.fitting,5000)
                compare(subject.areaBounds,subject.selectedBounds())
                verify(subject.selectedCoordinates().length > 0)
            }
        }
    }
}
