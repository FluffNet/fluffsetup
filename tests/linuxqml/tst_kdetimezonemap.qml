import QtQuick
import QtQuick.Controls
import QtTest
import QtPositioning
import "../../qml"

Item {
    width: 1000
    height: 600
    KdeTimezoneMap {
        id: map
        anchors.fill: parent
        zone: "Asia/Jerusalem"
        center: [31.7666667, 35.2333333]
        onZoneSelected: id => zone = id
    }
    SignalSpy { id: selection; target: map; signalName: "zoneSelected" }
    TestCase {
        name: "InstalledKdeTimezoneMap"
        when: windowShown
        function init() {
            failOnWarning(/.?/)
            map.zone = "Asia/Jerusalem"
            map.center = [31.7666667, 35.2333333]
            tryVerify(() => map.mapItem !== null, 5000)
            tryVerify(() => map.mapItem.mapReady, 5000)
            tryVerify(() => map.fittedZone === map.zone && !map.fitting, 5000)
        }
        function items(item) {
            let result = [item]
            for (let child of item.children) result = result.concat(items(child))
            return result
        }
        function borderFits(subject) {
            const coordinates = subject.selectedCoordinates()
            if (!coordinates.length) return false
            return coordinates.every(coordinate => {
                const point = subject.mapItem.fromCoordinate(coordinate, false)
                return Number.isFinite(point.x) && Number.isFinite(point.y)
                    && point.x >= subject.fitMargin - 1 && point.x <= subject.mapItem.width - subject.fitMargin + 1
                    && point.y >= subject.fitMargin - 1 && point.y <= subject.mapItem.height - subject.fitMargin + 1
            })
        }
        function test_initial_highlight_and_center() {
            tryVerify(() => map.mapItem !== null, 5000)
            tryVerify(() => map.mapItem.mapReady, 5000)
            compare(map.highlightedZone, "Asia/Jerusalem")
            verify(borderFits(map))
            compare(map.mapItem.zoomLevel, map.mapItem.maximumZoomLevel)
            verify(!items(map).some(item => item.chooseText !== undefined && item.currentText !== undefined && item.visible))
        }
        function test_external_selection_centers_without_echoing_selection() {
            selection.clear()
            map.center = [40.7141667, -74.0063889]
            map.zone = "America/New_York"
            tryCompare(map, "highlightedZone", "America/New_York")
            tryVerify(() => map.fittedZone === map.zone && !map.fitting)
            verify(borderFits(map))
            compare(selection.count, 0)
        }
        function test_empty_and_utc_choices_keep_hidden_combos_out_of_the_way() {
            for (let zone of ["", "UTC", "Asia/Jerusalem"]) {
                map.zone = zone
                tryCompare(map, "highlightedZone", zone)
                wait(150)
                verify(!items(map).some(item => item.activated !== undefined && item.currentText !== undefined && item.visible))
            }
        }
        function test_map_click_selects_the_area_and_mouse_drag_pans() {
            selection.clear()
            // Explore away from the close initial fit before clicking France.
            map.mapItem.zoomLevel = 2
            wait(200)
            const point = map.mapItem.fromCoordinate(QtPositioning.coordinate(48.8566, 2.3522), false)
            verify(map.surface.availableMapTimeZones.includes("Europe/Paris"))
            verify(map.surface.validTimeZones.includes("Europe/Paris"))
            verify(point.x > 0 && point.x < map.width && point.y > 0 && point.y < map.height)
            mouseClick(map.mapItem, point.x, point.y)
            tryVerify(() => selection.count > 0)
            compare(selection.signalArguments[0][0], "Europe/Paris")
            compare(map.highlightedZone, "Europe/Paris")
            tryVerify(() => map.fittedZone === map.zone && !map.fitting)
            const before = map.mapItem.center.longitude
            mouseDrag(map.mapItem, 500, 200, 100, 30)
            tryVerify(() => Math.abs(map.mapItem.center.longitude - before) > 0.01)
        }
        function test_zoom_buttons_wheel_and_touch_pinch() {
            const buttons = items(map).filter(item => item.clicked !== undefined && item.icon !== undefined && ["zoom-in-map-symbolic", "zoom-out-map-symbolic"].includes(item.icon.name))
            compare(buttons.length, 2)
            const plus = buttons.find(item => item.icon.name === "zoom-in-map-symbolic")
            const minus = buttons.find(item => item.icon.name === "zoom-out-map-symbolic")
            const before = map.mapItem.zoomLevel
            mouseClick(minus)
            tryVerify(() => map.mapItem.zoomLevel < before)
            mouseClick(plus)
            tryVerify(() => Math.abs(map.mapItem.zoomLevel - before) < 0.05)
            mouseWheel(map.mapItem, 400, 120, 0, -120)
            tryVerify(() => map.mapItem.zoomLevel < before)
            // Qt's native pinch scale reserves one zoom level of headroom.
            // Start farther below KDE's tile ceiling to exercise a real pinch,
            // rather than mistaking a tap-triggered re-fit for pinch zooming.
            map.mapItem.zoomLevel = map.mapItem.maximumZoomLevel - 2
            wait(200)
            const touch = touchEvent(map.mapItem)
            const start = map.mapItem.zoomLevel
            touch.press(0, map.mapItem, 430, 190).press(1, map.mapItem, 530, 190).commit()
            wait(20)
            for (let step = 1; step <= 6; step++) {
                touch.move(0, map.mapItem, 430 - step * 10, 190).move(1, map.mapItem, 530 + step * 10, 190).commit()
                wait(20)
            }
            tryVerify(() => map.mapItem.zoomLevel > start + 0.1)
            touch.release(0, map.mapItem).release(1, map.mapItem).commit()
            verify(map.mapItem.zoomLevel <= map.mapItem.maximumZoomLevel + 0.01)
        }
        function test_zoom_button_design_and_limits() {
            const plus = findChild(map, "fluff-timezone-zoom-in")
            const minus = findChild(map, "fluff-timezone-zoom-out")
            verify(plus && minus)
            map.mapItem.zoomLevel = map.mapItem.maximumZoomLevel - 1
            mouseMove(map, 20, 20)
            for (const button of [plus, minus]) {
                const fill = findChild(button, "timezone-zoom-fill")
                const label = findChild(button, "timezone-zoom-label")
                verify(fill && label)
                compare(fill.width, 44)
                compare(fill.height, 36)
                verify(button.width >= 48 && button.height >= 44)
                compare(fill.color, "#ffffff")
                compare(label.color, "#111111")
                compare(label.font.pixelSize, 24)
                compare(label.text, button === plus ? "+" : "-")
                verify(button.text.length > 0) // Retain KDE's localized tooltip/name.
                verify(!button.down)
            }
            mouseMove(minus)
            tryCompare(minus, "hovered", true)
            compare(findChild(minus, "timezone-zoom-fill").color, "#eceef1")
            mousePress(minus)
            tryCompare(minus, "down", true)
            compare(findChild(minus, "timezone-zoom-fill").color, "#e2e4e7")
            mouseRelease(minus)
            tryCompare(minus, "down", false)
            map.mapItem.zoomLevel = map.mapItem.maximumZoomLevel
            tryCompare(plus, "enabled", false)
            compare(findChild(plus, "timezone-zoom-fill").color, "#f4f4f5")
            compare(findChild(plus, "timezone-zoom-label").color, "#85888e")
            map.mapItem.zoomLevel = map.mapItem.minimumZoomLevel
            tryCompare(minus, "enabled", false)
            compare(findChild(minus, "timezone-zoom-fill").color, "#f4f4f5")
            compare(findChild(minus, "timezone-zoom-label").color, "#85888e")
        }
        function test_zoom_button_keyboard_and_touch() {
            const plus = findChild(map, "fluff-timezone-zoom-in")
            const minus = findChild(map, "fluff-timezone-zoom-out")
            const start = map.mapItem.maximumZoomLevel - 1
            map.mapItem.zoomLevel = start
            minus.forceActiveFocus()
            keyClick(Qt.Key_Space)
            tryVerify(() => map.mapItem.zoomLevel < start)
            const before = map.mapItem.zoomLevel
            touchEvent(plus).press(0, plus).commit().release(0, plus).commit()
            tryVerify(() => map.mapItem.zoomLevel > before)
        }
        function test_zoom_tooltips_follow_live_language_changes() {
            const plus = findChild(map,"fluff-timezone-zoom-in")
            const originalText = map.mapText
            const originalLanguage = map.language
            let translatedLanguage = "en"
            map.mapText = (context,source) => translatedLanguage+":"+source
            map.language = "en"
            compare(plus.text,"en:Zoom in")
            translatedLanguage = "he" // The backend catalog itself has no QML notification.
            map.language = "he"
            compare(plus.text,"he:Zoom in")
            map.mapText = originalText
            map.language = originalLanguage
        }
    }
}
