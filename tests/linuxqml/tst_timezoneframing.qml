import QtQuick
import QtTest
import "../../qml"

Item {
    width: 1100
    height: 900
    Component {
        id: mapComponent
        KdeTimezoneMap { center: [31.7666667, 35.2333333] }
    }
    Component {
        id: pickerComponent
        TimezonePicker {
            mapEnabled: true
            zones: [{code:"Asia/Jerusalem",label:"Jerusalem",suggested:true,center:[31.7666667,35.2333333]},
                    {code:"Europe/London",label:"London",center:[51.50853,-0.12574]}]
            selectedCode: "Asia/Jerusalem"
        }
    }
    TestCase {
        name: "TimezoneFraming"
        when: windowShown
        function init() { failOnWarning(/.?/) }
        function waitForFit(subject) {
            tryVerify(() => subject.mapItem && subject.mapItem.mapReady && subject.fittedZone === subject.zone && !subject.fitting, 5000)
        }
        function borderFits(subject, forFraming = false) {
            const coordinates = subject.selectedCoordinates(forFraming)
            verify(coordinates.length > 0)
            for (const coordinate of coordinates) {
                const point = subject.mapItem.fromCoordinate(coordinate, false)
                if (!Number.isFinite(point.x) || !Number.isFinite(point.y)
                        || point.x < subject.fitMargin - 1 || point.x > subject.mapItem.width - subject.fitMargin + 1
                        || point.y < subject.fitMargin - 1 || point.y > subject.mapItem.height - subject.fitMargin + 1) {
                    console.log("BORDER_FAILURE " + JSON.stringify({zone:subject.zone, coordinate:[coordinate.latitude,coordinate.longitude], point:point, center:[subject.mapItem.center.latitude,subject.mapItem.center.longitude],zoom:subject.mapItem.zoomLevel, width:subject.mapItem.width,height:subject.mapItem.height,margin:subject.fitMargin}))
                    return false
                }
            }
            return true
        }
        function test_cold_loader_autoselection_fits_without_a_second_selection() {
            const start = Date.now()
            const picker = createTemporaryObject(pickerComponent, parent, {width:1067,height:600})
            verify(picker)
            tryVerify(() => picker.mapItem !== null)
            waitForFit(picker.mapItem)
            console.log("COLD_FIT_MS " + (Date.now() - start) + " visible=" + picker.mapItem.surface.visiblePolygons.length + " total=" + picker.mapItem.surface.polygons.length)
            verify(Date.now() - start < 1000, "The first selected map view took over one second")
            compare(picker.mapItem.highlightedZone, "Asia/Jerusalem")
            compare(picker.mapItem.mapItem.zoomLevel, picker.mapItem.mapItem.maximumZoomLevel)
            verify(borderFits(picker.mapItem))
            wait(500) // Initial native-map resets must not undo the fitted camera.
            verify(borderFits(picker.mapItem))
            compare(picker.mapItem.mapItem.zoomLevel, picker.mapItem.mapItem.maximumZoomLevel)
        }
        function test_full_outline_data() {
            const rows = []
            const zones = [
                "Asia/Jerusalem", "Europe/London", "America/New_York",
                "Europe/Moscow", "Asia/Yakutsk", "Asia/Kamchatka", "Asia/Anadyr",
                "Asia/Shanghai", "Asia/Urumqi", "Australia/Adelaide",
                "Australia/Darwin", "Australia/Lord_Howe", "America/Manaus",
                "America/Sao_Paulo", "America/St_Johns", "America/Vancouver",
                "Pacific/Auckland", "Pacific/Fiji", "Asia/Kolkata",
                "Asia/Kathmandu", "Asia/Jakarta", "Asia/Pontianak",
                "Asia/Makassar", "Asia/Jayapura", "Pacific/Chatham",
                "America/Santiago", "America/Punta_Arenas", "Pacific/Easter",
                "Arctic/Longyearbyen", "America/Nuuk"
            ]
            for (const size of [[1067,252,"wide"],[675,255,"compact"],[275,720,"portrait"]])
                for (const zone of zones)
                    rows.push({tag:zone+"-"+size[2],zone:zone,width:size[0],height:size[1]})
            return rows
        }
        function test_full_outline(row) {
            const subject = createTemporaryObject(mapComponent, parent, {width:row.width,height:row.height,zone:row.zone})
            verify(subject)
            waitForFit(subject)
            verify(borderFits(subject), row.tag)
            verify(subject.viewportWidth <= subject.width + 1 && subject.viewportHeight <= subject.height + 1)
            verify(subject.mapItem.zoomLevel <= subject.mapItem.maximumZoomLevel + 0.01)
            if (row.zone === "Asia/Jerusalem") compare(subject.mapItem.zoomLevel, subject.mapItem.maximumZoomLevel)
        }
        function test_southern_area_fit_or_declared_polar_fallback_data() {
            const rows = []
            for (const size of [[1067,252,"wide"],[675,255,"compact"],[275,720,"portrait"]])
                for (const area of [
                    {zone:"Africa/Johannesburg",center:[-26.25,28]}
                ]) rows.push({tag:area.zone+"-"+size[2],width:size[0],height:size[1],zone:area.zone,center:area.center})
            return rows
        }
        function test_southern_area_fit_or_declared_polar_fallback(row) {
            const subject = createTemporaryObject(mapComponent,parent,{
                width:row.width,height:row.height,zone:row.zone,center:row.center
            })
            verify(subject)
            waitForFit(subject)
            compare(subject.highlightedZone,row.zone)
            if (!subject.polarCoverageLimited) {
                verify(borderFits(subject))
                return
            }
            // These native areas include Antarctic territory. Keep the explicit
            // deferred fallback, but use the selected city's actual center,
            // not the unrelated Jerusalem center of the generic fit fixture.
            const bounds = subject.selectedBounds()
            verify(bounds.topLeft.latitude > 85.05112878 || bounds.bottomRight.latitude < -85.05112878)
            compare(subject.areaBounds,null)
            compare(subject.viewportWidth,subject.width)
            compare(subject.viewportHeight,subject.height)
            verify(Math.abs(subject.mapItem.center.latitude-row.center[0]) < 0.01)
            verify(Math.abs(subject.mapItem.center.longitude-row.center[1]) < 0.01)
            compare(subject.mapItem.zoomLevel,subject.preferredZoom)
        }
        function test_perth_fits_every_australian_piece_data() {
            return [{tag:"wide",width:1067,height:252},{tag:"compact",width:675,height:255},
                {tag:"portrait",width:275,height:720}]
        }
        function test_perth_fits_every_australian_piece(data) {
            const subject = createTemporaryObject(mapComponent,parent,{
                width:data.width,height:data.height,zone:"Australia/Perth",center:[-31.95,115.85]
            })
            verify(subject)
            waitForFit(subject)
            const rows = subject.surface.polygons.filter(row => row.tzid === "Australia/Perth")
            const australian = rows.filter(row => row.bounds.topLeft.latitude >= -60)
            verify(australian.length > 0)
            // Retain all native data, but frame every non-Antarctic part.
            compare(subject.selectedCoordinates().length,rows.reduce((count,row) => count+row.coordinates.length,0))
            compare(subject.selectedCoordinates(true).length,australian.reduce((count,row) => count+row.coordinates.length,0))
            compare(subject.areaBounds,subject.selectedBounds(true))
            verify(subject.areaBounds.topLeft.latitude > -15)
            verify(subject.areaBounds.bottomRight.latitude < -35)
            verify(borderFits(subject,true))
            compare(subject.highlightedZone,"Australia/Perth")
            const balancedShift = subject.mapItem.center.longitude-subject.areaBounds.center.longitude
            verify(balancedShift >= -0.01,"Australian context moved in the wrong direction")
            if (data.width > data.height)
                verify(balancedShift > 1,"Wide Perth view did not balance the continent's right-heavy context")
            else
                verify(balancedShift < 0.1,"Portrait view shifted despite having no horizontal room")
            const polar = rows.some(row => row.bounds.bottomRight.latitude < -85.05112878)
            compare(subject.polarCoverageLimited,polar)
            if (polar) verify(subject.selectedCoordinates().length > subject.selectedCoordinates(true).length)
            wait(500)
            verify(borderFits(subject,true))
            subject.mapItem.zoomLevel -= 0.2
            subject.mapItem.pan(30,20)
            const zoom = subject.mapItem.zoomLevel
            const longitude = subject.mapItem.center.longitude
            wait(250)
            verify(Math.abs(subject.mapItem.zoomLevel-zoom) < 0.01)
            verify(Math.abs(subject.mapItem.center.longitude-longitude) < 0.01)
        }
        function test_coyhaique_uses_new_geometry_or_explicit_point_fallback() {
            const subject = createTemporaryObject(mapComponent,parent,{
                width:1067,height:252,zone:"America/Coyhaique",center:[-45.5667,-72.0667]
            })
            verify(subject)
            waitForFit(subject)
            compare(subject.highlightedZone,"America/Coyhaique")
            if (subject.surface.availableMapTimeZones.includes(subject.zone)) {
                verify(borderFits(subject))
            } else {
                compare(subject.areaBounds,null)
                verify(Math.abs(subject.mapItem.center.latitude+45.5667) < 0.01)
                verify(Math.abs(subject.mapItem.center.longitude+72.0667) < 0.01)
                compare(subject.mapItem.zoomLevel,subject.preferredZoom)
            }
        }
        function test_resize_refits_but_manual_pan_and_zoom_are_not_reset() {
            const subject = createTemporaryObject(mapComponent, parent, {width:1067,height:252,zone:"Europe/Moscow"})
            waitForFit(subject)
            verify(borderFits(subject))
            subject.width = 275; subject.height = 720
            waitForFit(subject)
            verify(borderFits(subject))
            subject.mapItem.zoomLevel -= 0.4
            subject.mapItem.pan(45,20)
            const zoom = subject.mapItem.zoomLevel
            const longitude = subject.mapItem.center.longitude
            wait(800)
            verify(Math.abs(subject.mapItem.zoomLevel - zoom) < 0.01)
            verify(Math.abs(subject.mapItem.center.longitude - longitude) < 0.01)
        }
        function test_page_navigation_reuses_native_map() {
            const picker = createTemporaryObject(pickerComponent,parent,{width:1067,height:600})
            tryVerify(() => picker.mapItem !== null)
            const adapter = picker.mapItem
            waitForFit(adapter)
            for (let iteration = 0; iteration < 20; ++iteration) {
                picker.visible = false
                picker.selectedCode = iteration % 2 ? "Asia/Jerusalem" : "Europe/London"
                wait(1)
                picker.visible = true
                compare(picker.mapItem,adapter)
                waitForFit(adapter)
                compare(adapter.highlightedZone,picker.selectedCode)
                verify(borderFits(adapter))
            }
        }
        function test_hidden_page_loads_only_on_first_visit() {
            const picker = createTemporaryObject(pickerComponent,parent,{width:1067,height:600,visible:false})
            wait(250)
            compare(picker.mapRequested,false)
            compare(picker.mapItem,null)
            picker.visible = true
            tryVerify(() => picker.mapItem !== null)
            const adapter = picker.mapItem
            waitForFit(adapter)
            picker.visible = false
            wait(250)
            compare(picker.mapItem,adapter)
        }
        function test_every_installed_area_fits_the_wide_view() {
            const subject = createTemporaryObject(mapComponent, parent, {width:1067,height:252,zone:"Asia/Jerusalem"})
            waitForFit(subject)
            const zones = subject.surface.availableMapTimeZones
            verify(zones.length > 100)
            const failures = []
            const deferred = []
            for (const zone of zones) {
                subject.zone = zone
                waitForFit(subject)
                if (subject.polarCoverageLimited) {
                    deferred.push(zone)
                    if (subject.areaBounds) verify(borderFits(subject,true),zone+" focused outline")
                    else {
                        compare(subject.viewportWidth, subject.width)
                        compare(subject.viewportHeight, subject.height)
                    }
                } else if (!borderFits(subject)) failures.push(zone)
            }
            console.log("Verified complete borders for " + (zones.length - deferred.length) + " installed areas; deferred polar coverage: " + deferred.join(", "))
            compare(failures.join(", "), "")
        }
    }
}
