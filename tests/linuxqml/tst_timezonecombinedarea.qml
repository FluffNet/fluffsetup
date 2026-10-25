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
            zone: "Asia/Jerusalem"
            onZoneSelected: id => {
                host.selections = host.selections.concat(id)
                zone = id
            }
        }
    }
    TestCase {
        name: "TimezoneCombinedArea"
        when: windowShown
        function init() { failOnWarning(/.?/); host.selections = [] }
        function ready(subject) {
            verify(subject)
            tryVerify(() => subject.mapItem.mapReady && !subject.fitting
                && subject.fittedZone === subject.zone, 5000)
            tryVerify(() => subject.surface.visiblePolygons.length > 0, 5000)
            wait(100)
            verify(grabImage(subject).width > 0)
        }
        function row(id, points) {
            const coordinates = points.map(point => QtPositioning.coordinate(...point))
            return {tzid:id, coordinates, shape:QtPositioning.polygon(coordinates),
                bounds:QtPositioning.rectangle(coordinates)}
        }
        function test_join_preserves_native_vertices_data() {
            const data = []
            for (const reversed of [false, true]) for (const closed of [false, true])
                for (const swapped of [false, true])
                    data.push({tag:[reversed,closed,swapped].join("-"),reversed,closed,swapped})
            return data
        }
        function test_join_preserves_native_vertices(data) {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            const first = [[30,34],[30,35],[31,35],[31,34]]
            let second = [[30,35],[30,36],[31,36],[31,35]]
            if (data.reversed) second.reverse()
            if (data.closed) { first.push(first[0]); second.push(second[0]) }
            const rows = [row("Asia/Jerusalem",first),row("Asia/Gaza",second)]
            const untouched = row("Europe/London",[[50,0],[50,1],[51,0]])
            if (data.swapped) rows.reverse()
            const result = subject.surface.joinJerusalemAreas(rows.concat(untouched))
            compare(result.length,2)
            compare(result[1],untouched)
            const joined = result[0]
            compare(joined.tzid,"Asia/Jerusalem")
            compare(joined.coordinates.length,6)
            for (const point of [[30.5,34.5],[30.5,35],[30.5,35.5]])
                verify(joined.shape.contains(QtPositioning.coordinate(...point)))
            verify(!joined.shape.contains(QtPositioning.coordinate(30.5,36.5)))
            for (const point of joined.coordinates)
                verify(rows.some(row => row.coordinates.some(native => native.latitude === point.latitude
                    && native.longitude === point.longitude)),"Union invented a boundary vertex")
            const edges = joined.coordinates.map((point,index) => [point,joined.coordinates[(index+1)%joined.coordinates.length]])
            verify(!edges.some(edge => edge.every(point => point.longitude === 35)),"Internal shared border remains")
        }
        function test_contained_detached_and_holed_pieces() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            const outer = row("Asia/Jerusalem",[[30,34],[30,36],[32,36],[32,34]])
            const inner = row("Asia/Hebron",[[30.5,34.5],[30.5,35],[31,35],[31,34.5]])
            for (const rows of [[inner,outer],[outer,inner],[outer,outer]]) {
                const result = subject.surface.joinJerusalemAreas(rows)
                compare(result.length,1)
                compare(result[0].coordinates,outer.coordinates)
            }
            const detached = row("Asia/Gaza",[[30,37],[30,38],[31,37]])
            const pieces = subject.surface.joinJerusalemAreas([outer,detached])
            compare(pieces.length,2)
            verify(pieces.every(piece => piece.tzid === "Asia/Jerusalem"))
            verify(pieces.every(piece => !piece.shape.contains(QtPositioning.coordinate(30.5,36.5))))
            const holed = row("Asia/Hebron",[[30,34],[30,36],[32,36],[32,34]])
            holed.shape.addHole([[30.5,34.5],[30.5,35],[31,35],[31,34.5]].map(point => QtPositioning.coordinate(...point)))
            const preserved = subject.surface.joinJerusalemAreas([holed])
            compare(preserved.length,1)
            compare(preserved[0].shape.holesCount(),1)
            verify(!preserved[0].shape.contains(QtPositioning.coordinate(30.75,34.75)))
        }
        function test_installed_combined_area_data() {
            return [{tag:"wide",width:1067,height:252},
                {tag:"compact",width:675,height:255},
                {tag:"portrait",width:275,height:600}]
        }
        function test_refined_border_is_shared_and_guarded_data() {
            const data = []
            for (const reversed of [false,true]) for (const closed of [false,true])
                for (const rotated of [false,true])
                    data.push({tag:[reversed,closed,rotated].join("-"),reversed,closed,rotated})
            return data
        }
        function test_refined_border_is_shared_and_guarded(data) {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252})
            ready(subject)
            function path(points) {
                if (data.reversed) points.reverse()
                if (data.rotated) points = points.slice(2).concat(points.slice(0,2))
                if (data.closed) points.push(points[0])
                return points
            }
            const jerusalem = row("Asia/Jerusalem",path([[31.466843,34.054691],[31.220129,34.267499],
                [29.45338,34.921211],[29.5,35.1],[31.7,35.1]]))
            const cairo = row("Africa/Cairo",path([[29.45338,34.921211],[31.220129,34.267499],
                [31.466843,34.054691],[31.3,33.5],[29.3,33.5]]))
            const unrelated = row("Europe/London",[[50,0],[50,1],[51,0]])
            const original = [unrelated,jerusalem,cairo]
            const result = subject.surface.refineEgyptBorder(original)
            compare(result.length,3)
            compare(result[0],unrelated)
            const border = result[1].coordinates.slice(0,38)
            const other = result[2].coordinates.slice(0,38).reverse()
            compare(border,other,"Neighboring areas have different shared borders")
            for (const latitude of [30.421142,30.530334]) {
                verify(result[1].shape.contains(QtPositioning.coordinate(latitude,34.55)))
                verify(!result[2].shape.contains(QtPositioning.coordinate(latitude,34.55)))
                verify(result[2].shape.contains(QtPositioning.coordinate(latitude,34.48)))
                verify(!result[1].shape.contains(QtPositioning.coordinate(latitude,34.48)))
            }
            const onlyOneSide = [unrelated,jerusalem]
            compare(subject.surface.refineEgyptBorder(onlyOneSide),onlyOneSide)
            const newer = row("Africa/Cairo",[[29.49,34.89],[30.5,34.5],[31.31,34.2],[31.3,33.5],[29.3,33.5]])
            const improved = [unrelated,jerusalem,newer]
            compare(subject.surface.refineEgyptBorder(improved),improved,"Improved upstream geometry was overwritten")
        }
        function test_installed_refined_border_data() { return test_installed_combined_area_data() }
        function test_installed_refined_border(data) {
            const subject = createTemporaryObject(mapComponent,host,{width:data.width,height:data.height})
            ready(subject)
            const surface = subject.surface
            const jerusalem = surface.polygons.find(row => row.tzid === "Asia/Jerusalem")
            const cairo = surface.polygons.find(row => row.tzid === "Africa/Cairo")
            verify(jerusalem && cairo)
            compare(jerusalem.coordinates.slice(0,38),cairo.coordinates.slice(0,38).reverse())
            // These points distinguish the refined Negev bend from KDE's
            // previous straight edge, without relying on subpixel mouse input.
            for (const sample of [
                {point:[30.421142,34.54],zone:"Asia/Jerusalem"},
                {point:[30.421142,34.51],zone:"Africa/Cairo"},
                {point:[31.28,34.26],zone:"Asia/Jerusalem"},
                {point:[31.28,34.20],zone:"Africa/Cairo"}
            ]) compare(surface.timezoneAt(QtPositioning.coordinate(...sample.point)),sample.zone)
            for (const sample of [
                {point:[30.53,34.65],zone:"Asia/Jerusalem"},
                {point:[30.53,34.35],zone:"Africa/Cairo"}
            ]) {
                subject.zone = "Asia/Jerusalem"
                ready(subject)
                const point = subject.mapItem.fromCoordinate(QtPositioning.coordinate(...sample.point),false)
                const before = host.selections.length
                mouseClick(subject.mapItem,point.x,point.y)
                tryVerify(() => host.selections.length === before+1)
                compare(host.selections[before],sample.zone)
                ready(subject)
                for (const coordinate of subject.selectedCoordinates()) {
                    const fitted = subject.mapItem.fromCoordinate(coordinate,false)
                    verify(fitted.x >= subject.fitMargin-1 && fitted.x <= subject.mapItem.width-subject.fitMargin+1)
                    verify(fitted.y >= subject.fitMargin-1 && fitted.y <= subject.mapItem.height-subject.fitMargin+1)
                }
            }
        }
        function test_installed_combined_area(data) {
            const subject = createTemporaryObject(mapComponent,host,{width:data.width,height:data.height})
            ready(subject)
            const surface = subject.surface
            verify(!surface.availableMapTimeZones.includes("Asia/Gaza"))
            verify(!surface.availableMapTimeZones.includes("Asia/Hebron"))
            const combined = surface.polygons.filter(row => row.tzid === "Asia/Jerusalem")
            compare(combined.length,1,"Internal outlines were not dissolved")
            const sourceIds = []
            function visit(node) {
                if (node.properties?.tzid) sourceIds.push(node.properties.tzid)
                for (let index = 0; index < (node.data?.length || 0); ++index) visit(node.data[index])
            }
            for (let index = 0; index < surface.geometryData.model.length; ++index)
                visit(surface.geometryData.model[index])
            verify(sourceIds.includes("Asia/Gaza") && sourceIds.includes("Asia/Hebron"),
                "Installed geometry must not be changed")
            for (const coordinate of combined[0].coordinates) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1)
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1)
            }
            for (const sample of [[31.4,34.3],[31.9,35.22],[32.1,34.8]]) {
                const coordinate = QtPositioning.coordinate(...sample)
                compare(surface.timezoneAt(coordinate),"Asia/Jerusalem")
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                const before = host.selections.length
                mouseClick(subject.mapItem,point.x,point.y)
                tryVerify(() => host.selections.length === before+1)
                compare(host.selections[before],"Asia/Jerusalem")
            }
            const before = host.selections.length
            surface.chooseTimezone("Asia/Gaza")
            surface.chooseTimezone("Asia/Hebron")
            compare(host.selections.length,before)
            for (const zoom of [2,4.9]) {
                subject.mapItem.zoomLevel = zoom
                wait(200)
                surface.updateViewport()
                compare(surface.timezoneAt(QtPositioning.coordinate(31.4,34.3)),"Asia/Jerusalem")
                compare(surface.timezoneAt(QtPositioning.coordinate(31.9,35.22)),"Asia/Jerusalem")
            }
            verify(surface.timezoneAt(QtPositioning.coordinate(31.9,35.85)) !== "Asia/Jerusalem")
        }
    }
}
