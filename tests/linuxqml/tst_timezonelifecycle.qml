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
            onZoneSelected: id => { host.selections.push(id); zone = id }
        }
    }
    TestCase {
        name: "TimezoneLifecycle"
        when: windowShown
        function init() { failOnWarning(/.?/); host.selections = [] }
        function waitForFit(subject) {
            tryVerify(() => subject.mapItem.mapReady && subject.fittedZone === subject.zone && !subject.fitting,5000)
            tryVerify(() => subject.selectedCoordinates().length > 0,5000)
        }
        function test_display_model_resolves_ids_without_visual_parents() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252,zone:"Europe/Paris"})
            waitForFit(subject)
            const shape = QtPositioning.polygon([QtPositioning.coordinate(1,1),QtPositioning.coordinate(1,2),QtPositioning.coordinate(2,1)])
            const rows = subject.surface.polygonRows([{type:"FeatureCollection",data:[
                {type:"MultiPolygon",properties:{tzid:"Europe/Paris"},data:[{type:"Polygon",data:shape},{type:"Polygon",data:shape}]},
                {type:"Polygon",properties:{tzid:"Asia/Jerusalem"},data:shape},
                {type:"GeometryCollection",properties:{tzid:"Asia/Tokyo"},data:[{type:"Polygon",data:shape}]},
                {type:"Polygon",data:shape},null
            ]}])
            compare(rows.map(row=>row.tzid).join(","),"Europe/Paris,Europe/Paris,Asia/Jerusalem,Asia/Tokyo")
            compare(subject.surface.polygonRows(null).length,0)
            subject.surface.chooseTimezone("")
            subject.surface.chooseTimezone("Not/A_Real_Zone")
            compare(host.selections.length,0)
            subject.surface.shuttingDown = true
            subject.surface.chooseTimezone("Asia/Jerusalem")
            compare(host.selections.length,0)
        }
        function test_detached_polygon_keeps_its_timezone_and_geometry() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252,zone:"Europe/Paris"})
            waitForFit(subject)
            const polygons = Array.from(subject.mapItem.mapItems).filter(item=>item.tzid === "Europe/Paris")
            verify(polygons.length > 1) // Mainland and detached island pieces.
            for (const polygon of polygons) {
                const previousParent = polygon.parent
                const count = polygon.path.length
                subject.mapItem.removeMapItem(polygon)
                polygon.parent = null
                wait(1)
                compare(polygon.tzid,"Europe/Paris")
                compare(polygon.path.length,count)
                compare(polygon.selected,true)
                subject.zone = "Asia/Jerusalem"
                compare(polygon.selected,false)
                subject.zone = "Europe/Paris"
                polygon.parent = previousParent
                if (!Array.from(subject.mapItem.mapItems).includes(polygon)) subject.mapItem.addMapItem(polygon)
            }
            waitForFit(subject)
            compare(host.selections.length,0)
        }
        function test_clear_reload_retains_selection() {
            const zones = ["Asia/Jerusalem","Europe/Paris","Asia/Tokyo","Australia/Sydney","Pacific/Fiji"]
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252,zone:zones[0]})
            waitForFit(subject)
            const count = subject.surface.polygons.length
            const ids = subject.surface.availableMapTimeZones.join(",")
            verify(count > 400)
            for (let iteration = 0; iteration < 25; ++iteration) {
                subject.zone = zones[iteration % zones.length]
                subject.surface.hoveredTimeZone = subject.zone
                subject.surface.geometryData.clear()
                compare(subject.surface.hoveredTimeZone,"")
                tryCompare(subject.surface,"polygons",[])
                tryVerify(() => subject.mapItem.mapItems.length === 0)
                compare(subject.highlightedZone,subject.zone)
                verify(subject.surface.geometryData.open())
                tryVerify(() => subject.surface.polygons.length === count)
                waitForFit(subject)
                compare(subject.surface.availableMapTimeZones.join(","),ids)
                subject.surface.chooseTimezone(zones[(iteration+1) % zones.length])
                compare(host.selections.length,iteration+1)
                compare(subject.zone,zones[(iteration+1) % zones.length])
                waitForFit(subject)
            }
        }
        function test_zero_viewport_recovers_without_changing_selection() {
            const subject = createTemporaryObject(mapComponent,host,{width:1067,height:252,zone:"Europe/Paris"})
            waitForFit(subject)
            const count = subject.surface.polygons.length
            subject.width = 0; subject.height = 0
            wait(250)
            compare(subject.zone,"Europe/Paris")
            compare(subject.surface.polygons.length,count)
            subject.width = 275; subject.height = 720
            waitForFit(subject)
            compare(host.selections.length,0)
        }
        function test_creation_interruption_and_destruction_data() {
            return [{tag:"wide",width:1067,height:252},{tag:"compact",width:675,height:255},{tag:"portrait",width:275,height:720}]
        }
        function test_creation_interruption_and_destruction(data) {
            const zones = ["Europe/Paris","Asia/Jerusalem","Asia/Tokyo","Australia/Sydney","Pacific/Fiji"]
            for (let iteration = 0; iteration < 40; ++iteration) {
                const subject = mapComponent.createObject(host,{width:data.width,height:data.height,zone:zones[iteration % zones.length]})
                verify(subject)
                if (iteration % 3 === 0) {
                    waitForFit(subject)
                    subject.surface.chooseTimezone(zones[(iteration+1) % zones.length])
                    compare(subject.zone,zones[(iteration+1) % zones.length])
                } else {
                    subject.width += 10
                    subject.zone = zones[(iteration+2) % zones.length]
                    wait(iteration % 2 ? 1 : 50)
                }
                const emitted = host.selections.length
                subject.zone = zones[(iteration+3) % zones.length]
                subject.destroy()
                wait(10) // Drain deferred deletion while failOnWarning is active.
                compare(host.selections.length,emitted)
            }
        }
    }
}
