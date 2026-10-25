import QtQuick
import QtTest
import QtPositioning
import "../../qml"

Item {
    id: host
    width: 1100
    height: 900
    property var selections: []
    // Regional city IDs/coordinates from the installed IANA zone.tab. Inland
    // click samples avoid ambiguous coastal pixels in a low-zoom overview.
    readonly property var countries: [
        {region:"VN",zones:[{code:"Asia/Ho_Chi_Minh",center:[10.75,106.6667],point:[12,108]}]},
        {region:"KR",zones:[{code:"Asia/Seoul",center:[37.55,126.9667],point:[36.35,127.38]}]},
        {region:"PH",zones:[{code:"Asia/Manila",center:[14.5867,120.9678],point:[15.5,121]}]},
        {region:"ID",zones:[
            {code:"Asia/Jakarta",center:[-6.1667,106.8],point:[-6.9,107.6]},
            {code:"Asia/Pontianak",center:[-0.0333,109.3333],point:[-0.5,111]},
            {code:"Asia/Makassar",center:[-5.1167,119.4],point:[-3.5,120]},
            {code:"Asia/Jayapura",center:[-2.5333,140.7],point:[-4.08,138.95]}
        ]},
        {region:"JP",zones:[{code:"Asia/Tokyo",center:[35.6544,139.7447],point:[36,138]}]},
        {region:"TW",zones:[{code:"Asia/Taipei",center:[25.05,121.5],point:[23.8,121]}]},
        {region:"MY",zones:[
            {code:"Asia/Kuala_Lumpur",center:[3.1667,101.7],point:[4,102]},
            {code:"Asia/Kuching",center:[1.55,110.3333],point:[3,114]}
        ]},
        {region:"TH",zones:[{code:"Asia/Bangkok",center:[13.75,100.5167],point:[15,101]}]},
        {region:"MN",zones:[
            {code:"Asia/Ulaanbaatar",center:[47.9167,106.8833],point:[47,106]},
            {code:"Asia/Hovd",center:[48.0167,91.65],point:[48,91.5]}
        ]},
        {region:"FJ",zones:[{code:"Pacific/Fiji",center:[-18.1333,178.4167],point:[-17.8,178]}]},
        {region:"KI",zones:[
            {code:"Pacific/Tarawa",center:[1.4167,173],point:[1.5,173]},
            {code:"Pacific/Kanton",center:[-2.7833,-171.7167],point:[-2.8195,-171.655]},
            {code:"Pacific/Kiritimati",center:[1.8667,-157.3333],point:[1.872,-157.3657]}
        ]},
        {region:"PT",zones:[
            {code:"Europe/Lisbon",center:[38.7167,-9.1333],point:[39.5,-8]},
            {code:"Atlantic/Madeira",center:[32.6333,-16.9],point:[32.75,-16.9]},
            {code:"Atlantic/Azores",center:[37.7333,-25.6667],point:[37.8,-25.5]}
        ]}
    ]
    Component {
        id: pickerComponent
        TimezonePicker {
            mapEnabled: true
            onZoneSelected: id => {
                host.selections = host.selections.concat(id)
                selectedCode = id
            }
        }
    }
    TestCase {
        name: "TimezoneCountries"
        when: windowShown
        function init() { failOnWarning(/.?/); host.selections = [] }
        function cases() {
            const rows = []
            for (const country of host.countries)
                for (const size of [[1067,650,"wide"],[675,650,"compact"],[275,900,"portrait"]])
                    rows.push(Object.assign({},country,{tag:country.region+"-"+size[2],width:size[0],height:size[1]}))
            return rows
        }
        function createPicker(data) {
            const zones = data.zones.map(zone => Object.assign({label:zone.code,suggested:true},zone))
            zones.push({code:"Europe/London",label:"London",center:[51.5,0]},
                {code:"UTC",label:"UTC"})
            return createTemporaryObject(pickerComponent,host,{
                width:data.width,height:data.height,region:data.region,zones:zones,
                selectedCode:data.zones.length === 1 ? data.zones[0].code : ""
            })
        }
        function ready(picker) {
            verify(picker)
            tryVerify(() => picker.mapItem !== null,5000)
            tryVerify(() => picker.mapItem.mapItem.mapReady && !picker.mapItem.fitting
                && picker.mapItem.fittedZone === picker.selectedCode,5000)
            wait(100)
            verify(grabImage(picker).width > 0)
        }
        function fits(subject, coordinates) {
            verify(coordinates.length > 0)
            for (const coordinate of coordinates) {
                const point = subject.mapItem.fromCoordinate(coordinate,false)
                verify(Number.isFinite(point.x) && Number.isFinite(point.y))
                verify(point.x >= subject.fitMargin-1 && point.x <= subject.mapItem.width-subject.fitMargin+1,
                    "East/west outline clipped: "+coordinate)
                verify(point.y >= subject.fitMargin-1 && point.y <= subject.mapItem.height-subject.fitMargin+1,
                    "North/south outline clipped: "+coordinate)
            }
        }
        function test_initial_regional_fit_data() { return cases() }
        function test_initial_regional_fit(data) {
            const picker = createPicker(data)
            ready(picker)
            const subject = picker.mapItem
            verify(subject.areaBounds && subject.areaBounds.isValid,"No fitted regional overview for "+data.region)
            const ids = data.zones.map(zone => zone.code)
            const coordinates = []
            for (const row of subject.surface.polygons)
                if (ids.includes(row.tzid))
                    for (const coordinate of row.coordinates) coordinates.push(coordinate)
            fits(subject,coordinates)
            compare(picker.canConfirm,data.zones.length === 1)
            compare(picker.filteredZones.length,data.zones.length+2)
            if (data.region === "FJ" || data.region === "KI")
                verify(Math.abs(subject.mapItem.center.longitude) > 150,"Date-line area centered on the wrong hemisphere")
            const center = subject.mapItem.center
            const zoom = subject.mapItem.zoomLevel
            wait(150)
            verify(Math.abs(subject.mapItem.center.longitude-center.longitude) < 0.01)
            verify(Math.abs(subject.mapItem.zoomLevel-zoom) < 0.01)
        }
        function test_vietnam_northern_shared_area_keeps_its_installed_id() {
            const picker = createPicker(Object.assign({},host.countries[0],{width:1067,height:650}))
            picker.zones = picker.zones.concat({code:"Asia/Bangkok",label:"Bangkok",center:[13.75,100.5167]})
            ready(picker)
            const subject = picker.mapItem
            subject.mapItem.zoomLevel = 2
            subject.mapItem.center = QtPositioning.coordinate(20,105)
            wait(200)
            verify(grabImage(picker).width > 0)
            const point = subject.mapItem.fromCoordinate(QtPositioning.coordinate(21.028,105.834),false)
            mouseClick(subject.mapItem,point.x,point.y)
            tryCompare(picker,"selectedCode","Asia/Bangkok")
            ready(picker)
            compare(subject.highlightedZone,"Asia/Bangkok")
            verify(picker.canConfirm)
            picker.searchText = "Bangkok"
            compare(picker.confirmationCode,"Asia/Bangkok")
        }
        function test_real_map_selections_data() { return cases() }
        function test_real_map_selections(data) {
            const picker = createPicker(data)
            for (const zone of data.zones) {
                // Return to the overview between independent region choices.
                picker.selectedCode = data.zones.length === 1 ? zone.code : ""
                ready(picker)
                const subject = picker.mapItem
                const point = subject.mapItem.fromCoordinate(QtPositioning.coordinate(...zone.point),false)
                verify(point.x > 1 && point.x < subject.mapItem.width-1
                    && point.y > 1 && point.y < subject.mapItem.height-1,zone.code+" click sample outside regional overview")
                const before = host.selections.length
                mouseClick(subject.mapItem,point.x,point.y)
                tryCompare(picker,"selectedCode",zone.code)
                tryCompare(host,"selections",host.selections.slice(0,before).concat(zone.code))
                ready(picker)
                compare(subject.highlightedZone,zone.code)
                fits(subject,subject.selectedCoordinates())
                verify(picker.canConfirm)
                picker.searchText = zone.code
                compare(picker.filteredZones.length,1)
                compare(picker.confirmationCode,zone.code)
                picker.confirm()
                compare(picker.selectedCode,zone.code)
                picker.searchText = ""
            }
        }
    }
}
