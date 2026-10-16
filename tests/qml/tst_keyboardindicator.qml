import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    id: surface
    property var originalLayouts
    width: 800
    height: 600
    TextField { id: input; x: 300; y: 100; width: 200 }
    KeyboardIndicator {
        id: indicator
        x: rtl ? surface.width - width - 24 : 24
        y: surface.height - height - 24
        layouts: [{id: "us", name: "U.S. English (QWERTY)"},
                  {id: "il", name: "Hebrew"},
                  {id: "de", name: "German (QWERTZ)"},
                  {id: "fr", name: "French (AZERTY)"}]
        activeIndex: 0
        onLayoutRequested: id => activeIndex = layouts.findIndex(layout => layout.id === id)
        onRestoreTypingFocus: input.forceActiveFocus()
    }
    SignalSpy { id: choiceSpy; target: indicator; signalName: "layoutRequested" }
    TestCase {
        name: "KeyboardIndicator"
        when: windowShown
        function initTestCase() { originalLayouts = indicator.layouts }
        function init() {
            indicator.menu.close()
            surface.width = 800
            surface.height = 600
            indicator.uiScale = 1
            indicator.popupAvailableWidth = 960
            indicator.popupHorizontalOffset = Qt.binding(() => indicator.rtl ? indicator.width - indicator.menu.width : 0)
            indicator.rtl = false
            indicator.visible = Qt.binding(() => indicator.hasMultipleLayouts)
            indicator.enabled = true
            indicator.activeIndex = 0
            indicator.layouts = originalLayouts.slice()
            input.text = ""
            input.forceActiveFocus()
            choiceSpy.clear()
            wait(40)
        }
        function test_mouse_touch_keyboard_data() {
            const rows = []
            for (let rtl of [false, true]) for (let scale of [1, 0.5])
                for (let input of ["mouse", "touch", "keyboard"])
                    rows.push({tag: (rtl ? "RTL" : "LTR") + scale + input, rtl, scale, input})
            return rows
        }
        function test_mouse_touch_keyboard(data) {
            indicator.rtl = data.rtl
            indicator.uiScale = data.scale
            wait(40)
            if (data.input === "mouse") mouseClick(indicator)
            else if (data.input === "touch") touchEvent(indicator).press(0, indicator).commit().release(0, indicator).commit()
            else { indicator.forceActiveFocus(); keyClick(Qt.Key_Space) }
            tryCompare(indicator.menu, "opened", true)
            const item = indicator.menu.itemAt(1)
            verify(item)
            if (data.input === "keyboard") { keyClick(Qt.Key_Down); keyClick(Qt.Key_Return) }
            else if (data.input === "mouse") mouseClick(item)
            else touchEvent(item).press(0, item).commit().release(0, item).commit()
            tryCompare(indicator, "activeIndex", 1)
            compare(choiceSpy.count, 1)
            compare(choiceSpy.signalArguments[0][0], "il")
            tryCompare(indicator.menu, "opened", false)
            tryCompare(input, "activeFocus", true)
            keyClick(Qt.Key_A)
            compare(input.text, "a")
            compare(indicator.layouts[0].id, "us")
        }
        function test_shortcut_status_and_escape() {
            indicator.activeIndex = 2
            compare(indicator.activeLayout.id, "de")
            mouseClick(indicator)
            tryCompare(indicator.menu, "opened", true)
            compare(indicator.menu.currentIndex, 2)
            verify(indicator.menu.itemAt(2).checked)
            keyClick(Qt.Key_Escape)
            tryCompare(indicator.menu, "opened", false)
            tryCompare(input, "activeFocus", true)
            compare(choiceSpy.count, 0)
        }
        function test_button_dimensions_do_not_depend_on_active_layout() {
            for (let scale of [1, 0.73, 0.75]) {
                indicator.uiScale = scale
                wait(30)
                const width = indicator.width, height = indicator.height
                for (let index = 0; index < originalLayouts.length; ++index) {
                    indicator.activeIndex = index
                    wait(20)
                    compare(indicator.width, width)
                    compare(indicator.height, height)
                    const name = findChild(indicator, "keyboard-indicator-name")
                    verify(name.paintedWidth <= name.width + 1)
                    verify(name.paintedHeight <= name.height + 1)
                    if (index === 1) compare(name.lineCount, 1)
                }
            }
        }
        function test_full_name_button_data() {
            return [{tag: "normal", scale: 1}, {tag: "small", scale: 0.73}, {tag: "portrait", scale: 0.75}]
        }
        function test_full_name_button(data) {
            indicator.uiScale = data.scale
            wait(40)
            const name = findChild(indicator, "keyboard-indicator-name")
            const button = findChild(indicator, "keyboard-indicator-button")
            compare(name.text, "\u2068U.S. English\n(QWERTY)\u2069")
            verify(indicator.width >= 111 * data.scale)
            verify(indicator.width <= 253 * data.scale)
            compare(button.height, indicator.height)
            verify(button.color.a > 0)
            verify(button.border.width > 0)
            indicator.layouts = [{id: "us:intl", name: "U.S. English (International)"},
                                 {id: "us:colemak_dh", name: "U.S. English (Colemak DH)"}]
            wait(40)
            compare(name.text, "\u2068U.S. English\n(International)\u2069")
            indicator.activeIndex = 1
            wait(40)
            compare(name.text, "\u2068U.S. English\n(Colemak DH)\u2069")
            verify(indicator.width <= 253 * data.scale)
            verify(name.paintedWidth <= name.width + 1)
            verify(name.paintedHeight <= name.height + 1)
            verify(indicator.Accessible.name.includes("Colemak DH"))
            indicator.layouts = [{id: "fr", name: "Französische Tastaturbelegung mit einer sehr langen vollständigen Bezeichnung (AZERTY)"}, originalLayouts[0]]
            indicator.activeIndex = 0
            wait(60)
            verify(name.lineCount > 1)
            verify(name.paintedWidth <= name.width + 1)
            verify(name.paintedHeight <= name.height + 1)
            verify(indicator.height > 44 * data.scale)
        }
        function test_both_enter_keys_data() {
            return [{tag: "Return", key: Qt.Key_Return}, {tag: "numpad Enter", key: Qt.Key_Enter}]
        }
        function test_keyboard_icon_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_keyboard_icon(data) {
            indicator.rtl = data.rtl
            const icon = findChild(indicator, "keyboard-indicator-icon")
            const name = findChild(indicator, "keyboard-indicator-name")
            tryCompare(icon, "status", Image.Ready)
            wait(40)
            verify(icon.source.toString().endsWith("/assets/keyboard-layout.svg"))
            compare(icon.width, 22)
            compare(icon.height, 22)
            if (data.rtl) verify(icon.x >= name.x + name.width - 1)
            else verify(icon.x + icon.width <= name.x + 1)
            indicator.enabled = false
            compare(icon.opacity, 0.5)
        }
        function test_both_enter_keys(data) {
            indicator.forceActiveFocus()
            keyClick(Qt.Key_Space)
            tryCompare(indicator.menu, "opened", true)
            keyClick(Qt.Key_Down)
            keyClick(data.key)
            tryCompare(indicator, "activeIndex", 1)
            tryCompare(input, "activeFocus", true)
        }
        function test_hidden_disabled_and_unknown_state() {
            indicator.activeIndex = -1
            compare(indicator.activeLayout, null)
            indicator.enabled = false
            mouseClick(indicator)
            compare(indicator.menu.opened, false)
            indicator.enabled = true
            mouseClick(indicator)
            tryCompare(indicator.menu, "opened", true)
            indicator.visible = false
            tryCompare(indicator.menu, "opened", false)
        }
        function test_single_layout_visibility_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_single_layout_visibility(data) {
            indicator.rtl = data.rtl
            indicator.layouts = [originalLayouts[0]]
            tryCompare(indicator, "visible", false)
            indicator.clicked()
            compare(indicator.menu.opened, false)
            indicator.layouts = originalLayouts.slice(0, 2)
            tryCompare(indicator, "visible", true)
            mouseClick(indicator)
            tryCompare(indicator.menu, "opened", true)
            indicator.layouts = [originalLayouts[0]]
            tryCompare(indicator, "visible", false)
            tryCompare(indicator.menu, "opened", false)
            tryCompare(input, "activeFocus", true)
            indicator.layouts = originalLayouts.slice(0, 2)
            tryCompare(indicator, "visible", true)
            compare(choiceSpy.count, 0)
        }
        function test_long_names_and_portrait_bounds_data() {
            return [{tag: "800x600-LTR", width: 800, height: 600, rtl: false, scale: 0.73},
                    {tag: "800x600-RTL", width: 800, height: 600, rtl: true, scale: 0.73},
                    {tag: "720x1280-LTR", width: 720, height: 1280, rtl: false, scale: 0.75},
                    {tag: "720x1280-RTL", width: 720, height: 1280, rtl: true, scale: 0.75}]
        }
        function test_sidebar_menu_width_data() {
            const rows = []
            for (let rtl of [false, true]) for (let scale of [1, 0.73, 0.75])
                rows.push({tag: (rtl ? "RTL" : "LTR") + scale, rtl, scale})
            return rows
        }
        function test_sidebar_menu_width(data) {
            indicator.rtl = data.rtl
            indicator.uiScale = data.scale
            const panelWidth = Math.round(300 * data.scale)
            const panelLeft = data.rtl ? surface.width - panelWidth : 0
            indicator.popupAvailableWidth = panelWidth
            indicator.popupHorizontalOffset = panelLeft - indicator.x
            indicator.layouts = [originalLayouts[0],
                {id: "fr", name: "Französische Tastaturbelegung mit einer sehr langen vollständigen Bezeichnung (AZERTY)"}]
            wait(60)
            // The label may change the button width and therefore its position.
            indicator.popupHorizontalOffset = panelLeft - indicator.x
            mouseClick(indicator)
            tryCompare(indicator.menu, "opened", true)
            wait(60)
            compare(indicator.menu.width, panelWidth)
            const menuPosition = indicator.menu.contentItem.mapToItem(surface, 0, 0)
            const menuLeft = menuPosition.x - indicator.menu.leftPadding
            verify(Math.abs(menuLeft - panelLeft) <= 1)
            verify(Math.abs(menuLeft + indicator.menu.width - panelLeft - panelWidth) <= 1)
            for (let i = 0; i < 2; ++i) {
                const row = indicator.menu.itemAt(i)
                const p = row.mapToItem(surface, 0, 0)
                verify(p.x >= panelLeft - 1 && p.x + row.width <= panelLeft + panelWidth + 1)
                for (let child of row.contentItem.children)
                    if (child.paintedHeight !== undefined) {
                        verify(child.paintedHeight <= child.height + 1)
                        if (i === 1 && child.text.length > 20) verify(child.lineCount > 1)
                    }
            }
            mouseClick(indicator.menu.itemAt(1))
            tryCompare(indicator, "activeIndex", 1)
            tryCompare(input, "activeFocus", true)
        }
        function test_long_names_and_portrait_bounds(data) {
            surface.width = data.width
            surface.height = data.height
            indicator.rtl = data.rtl
            indicator.uiScale = data.scale
            indicator.layouts = originalLayouts.map((layout, index) => index === 3
                ? {id: "fr", name: "Französische Tastaturbelegung mit einer sehr langen vollständigen Bezeichnung (AZERTY)"}
                : layout)
            mouseClick(indicator)
            tryCompare(indicator.menu, "opened", true)
            wait(60)
            for (let i = 0; i < 4; ++i) {
                const row = indicator.menu.itemAt(i)
                const position = row.mapToItem(surface, 0, 0)
                verify(position.x >= -1 && position.y >= -1)
                verify(position.x + row.width <= surface.width + 1)
                verify(position.y + row.height <= surface.height + 1)
                for (let child of row.contentItem.children)
                    if (child.paintedHeight !== undefined) verify(child.paintedHeight <= child.height + 1,
                        "Text clips: " + child.text + " painted=" + child.paintedHeight + " height=" + child.height)
            }
        }
    }
}
