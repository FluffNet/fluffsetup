import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    id: testSurface
    property var originalLayouts
    property var originalLabels
    width: 600
    height: 600
    KeyboardPicker {
        id: picker
        anchors.fill: parent
        layouts: [
            {id: "us", name: "U.S. English", search: "English (US)"},
            {id: "il", name: "עברית", search: "Hebrew עברית"},
            {id: "fr", name: "French", search: "Français AZERTY"},
            {id: "us:intl", name: "English (US, international)", search: "English intl"},
            {id: "de", name: "German", search: "Deutsch"},
            {id: "gb", name: "English (UK)", search: "British English"}
        ]
        selectedIds: ["us", "il"]
        onAddRequested: id => {
            if (!selectedIds.includes(id) && selectedIds.length < 4) selectedIds = selectedIds.concat([id])
        }
        onRemoveRequested: id => {
            if (selectedIds.length > 1) selectedIds = selectedIds.filter(entry => entry !== id)
        }
        onMoveRequested: (id, direction) => {
            const ids = selectedIds.slice()
            const index = ids.indexOf(id)
            const destination = index + direction
            if (index >= 0 && destination >= 0 && destination < ids.length) {
                const other = ids[destination]
                ids[destination] = ids[index]
                ids[index] = other
                selectedIds = ids
            }
        }
        onReorderRequested: (id, destination) => {
            const ids = selectedIds.slice()
            const source = ids.indexOf(id)
            if (source >= 0 && destination >= 0 && destination < ids.length) {
                ids.splice(source, 1)
                ids.splice(destination, 0, id)
                selectedIds = ids
            }
        }
    }
    SignalSpy { id: continueSpy; target: picker; signalName: "continueRequested" }
    SignalSpy { id: reorderSpy; target: picker; signalName: "reorderRequested" }
    TestCase {
        name: "KeyboardPicker"
        when: windowShown
        function initTestCase() {
            originalLayouts = picker.layouts
            originalLabels = picker.labels
        }
        function init() {
            picker.layouts = originalLayouts
            picker.labels = originalLabels
            testSurface.width = 600
            testSurface.height = 600
            picker.visible = true
            picker.enabled = true
            picker.cancelSelectedDrag()
            picker.resultsView.cancelFlick()
            picker.uiScale = 1
            picker.rtl = false
            picker.selectedIds = ["us", "il"]
            picker.searchText = ""
            picker.focusSearch()
            continueSpy.clear()
            reorderSpy.clear()
            wait(60)
        }
        function test_search_data() {
            return [{tag: "English", query: "international", expected: "us:intl"},
                    {tag: "native", query: "francais", expected: "fr"},
                    {tag: "code", query: " gb ", expected: "gb"},
                    {tag: "no-results", query: "zzzz", expected: ""}]
        }
        function test_missing_default_suggestions_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_missing_default_suggestions(data) {
            picker.rtl = data.rtl
            picker.layouts = [
                {id: "il", name: "Hebrew", group: "suggested"},
                {id: "us", name: "U.S. English", group: "other"},
                {id: "fr", name: "French", group: "other"}
            ]
            picker.selectedIds = ["us", "il"]
            wait(100)
            compare(picker.hasSuggestions, false)
            picker.removeRequested("il")
            tryCompare(picker, "hasSuggestions", true)
            wait(100)
            const heading = findChild(picker, "keyboard-section-suggested")
            verify(heading)
            compare(heading.text, "Suggested layouts")
            picker.resultsView.forceActiveFocus()
            keyClick(Qt.Key_Home)
            keyClick(Qt.Key_Return)
            tryCompare(picker, "hasSuggestions", false)
            verify(picker.selectedIds.includes("il"))
            picker.removeRequested("il")
            picker.searchText = "French"
            tryCompare(picker, "hasSuggestions", false)
            compare(picker.filteredLayouts[0].id, "fr")
        }
        function test_scrollbar_only_for_overflow_data() {
            const rows = []
            for (const rtl of [false,true]) for (const scale of [1,0.65])
                rows.push({tag:(rtl?"RTL":"LTR")+"-"+scale,rtl,scale})
            return rows
        }
        function test_scrollbar_only_for_overflow(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            const view = picker.resultsView
            const bar = findChild(picker,"keyboardResultsScrollBar")
            verify(bar)
            tryCompare(bar,"visible",false)
            const left = view.leftMargin, right = view.rightMargin
            picker.searchText = "zzzz"
            tryCompare(view,"count",0)
            tryCompare(bar,"visible",false)
            picker.searchText = "German"
            tryCompare(view,"count",1)
            tryCompare(bar,"visible",false)
            picker.searchText = ""
            picker.layouts = originalLayouts.concat(Array.from({length:50},(_,index) => ({
                id:"extra-"+index,name:"Extra layout "+index
            })))
            tryCompare(bar,"visible",true)
            wait(800)
            verify(bar.contentItem.opacity > 0,"An overflowing list's thumb faded away")
            compare(view.leftMargin,left)
            compare(view.rightMargin,right)
            const before = view.contentY
            mouseWheel(view,view.width/2,view.height/2,0,-240)
            tryVerify(() => view.contentY > before+10)
            wait(350)
            picker.searchText = "German"
            tryCompare(view,"count",1)
            tryCompare(bar,"visible",false)
            picker.searchText = "zzzz"
            tryCompare(view,"count",0)
            tryCompare(bar,"visible",false)
        }
        function test_search(data) {
            picker.searchText = data.query
            wait(30)
            compare(picker.filteredLayouts.length, data.expected.length ? 1 : 0)
            if (data.expected.length) compare(picker.filteredLayouts[0].id, data.expected)
        }
        function test_search_placeholder_without_hover_tooltip_data() {
            const languages = [
                {tag: "English", rtl: false, text: "Search keyboard layouts or use search bar to test keyboard layout"},
                {tag: "Hebrew", rtl: true, text: "חיפוש פריסות מקלדת או שימוש בשורת החיפוש לבדיקת פריסת המקלדת"}
            ]
            const rows = []
            for (const language of languages) for (const scale of [1, 0.65])
                rows.push({tag: language.tag + "-" + scale, rtl: language.rtl,
                           text: language.text, scale: scale})
            return rows
        }
        function test_search_placeholder_without_hover_tooltip(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            picker.labels = Object.assign({}, originalLabels, {search: data.text})
            const field = picker.searchInput
            const placeholder = findChild(picker, "keyboard-search-placeholder")
            verify(placeholder)
            compare(field.placeholderText, data.text)
            compare(placeholder.text, data.text)
            compare(placeholder.visible, true)
            mouseMove(field, field.width / 2, field.height / 2)
            tryCompare(field, "hovered", true)
            wait(800)
            compare(field.ToolTip.visible, false)
            compare(field.ToolTip.text, "")
            picker.searchText = "French"
            tryCompare(placeholder, "visible", false)
            wait(800)
            compare(field.ToolTip.visible, false)
            compare(picker.filteredLayouts.length, 1)
            compare(picker.filteredLayouts[0].id, "fr")
        }
        function test_enter_data() {
            return [{tag: "regular", key: Qt.Key_Return}, {tag: "numpad", key: Qt.Key_Enter}]
        }
        function test_live_search_in_english_and_selected_language_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_live_search_in_english_and_selected_language(data) {
            picker.rtl = data.rtl
            picker.selectedIds = ["us"]
            picker.layouts = [
                {id: "us", name: "אנגלית (ארצות הברית)", search: "English (US)"},
                {id: "il", name: "עברית", search: "Hebrew עברית"},
                {id: "fr:oss", name: "צרפתית (חלופית)", search: "French (alt.) צרפתית fr:oss"},
                {id: "de", name: "גרמנית", search: "German Deutsch גרמנית"}
            ]
            picker.focusSearch()
            for (const query of ["Hebrew", "עברית", "French", "צרפתית", "German", "גרמנית", "zzzz"]) {
                picker.searchInput.clear()
                for (let index = 0; index < query.length; ++index) {
                    picker.searchInput.insert(picker.searchInput.length, query[index])
                    wait(5)
                    const text = picker.searchInput.text
                    const expected = picker.layouts.filter(layout => layout.id !== "us" &&
                        picker.searchKey(layout.name + " " + layout.search + " " + layout.id).includes(picker.searchKey(text)))
                    compare(picker.filteredLayouts.map(layout => layout.id).join(","), expected.map(layout => layout.id).join(","))
                }
                compare(picker.selectedIds.join(","), "us")
            }
        }
        function test_enter(data) {
            keyClick(data.key)
            compare(continueSpy.count, 1)
            compare(picker.selectedIds.length, 2)
            picker.searchText = "international"
            wait(30)
            keyClick(data.key)
            compare(picker.selectedIds[2], "us:intl")
            compare(picker.filteredLayouts.length, 0)
        }
        function test_arrows_add_default_remove_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_arrows_add_default_remove(data) {
            picker.rtl = data.rtl
            keyClick(Qt.Key_Down)
            compare(picker.resultsView.activeFocus, true)
            compare(picker.resultsView.currentIndex, 0)
            keyClick(Qt.Key_Down)
            compare(picker.resultsView.currentIndex, 1)
            keyClick(Qt.Key_Enter)
            wait(40)
            compare(picker.selectedIds[2], "us:intl")
            const choose = findChild(picker, "move-up-il")
            verify(choose !== null)
            choose.forceActiveFocus()
            keyClick(Qt.Key_Space)
            wait(30)
            compare(picker.selectedIds.join(","), "il,us,us:intl")
            const currentDefault = findChild(picker, "drag-il")
            mouseClick(currentDefault)
            compare(findChild(picker, "selected-row-il").isDefault, true)
            const remove = findChild(picker, "remove-il")
            mouseClick(remove)
            wait(30)
            compare(picker.selectedIds.join(","), "us,us:intl")
            picker.selectedIds = ["us"]
            wait(30)
            compare(findChild(picker, "remove-us").enabled, false)
        }
        function test_limit_duplicates_and_selected_search() {
            picker.selectedIds = ["us", "il", "fr", "de"]
            picker.searchText = "international"
            wait(30)
            keyClick(Qt.Key_Return)
            compare(picker.selectedIds.length, 4)
            compare(picker.filteredLayouts.length, 1)
            picker.selectedIds = ["us"]
            picker.searchText = "U.S. English"
            wait(30)
            compare(picker.filteredLayouts.length, 0)
        }
        function test_reorder_data() { return [{tag: "regular-LTR", rtl: false, key: Qt.Key_Return}, {tag: "regular-RTL", rtl: true, key: Qt.Key_Return}, {tag: "numpad-LTR", rtl: false, key: Qt.Key_Enter}, {tag: "numpad-RTL", rtl: true, key: Qt.Key_Enter}] }
        function test_reorder(data) {
            picker.rtl = data.rtl
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(40)
            compare(findChild(picker, "move-up-us").enabled, false)
            compare(findChild(picker, "move-down-de").enabled, false)
            const up = findChild(picker, "move-up-fr")
            up.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Space)
            wait(40)
            compare(picker.selectedIds.join(","), "us,fr,il,de")
            verify(findChild(picker, "move-up-fr").activeFocus)
            keyClick(data.key)
            wait(40)
            compare(picker.selectedIds.join(","), "fr,us,il,de")
            compare(findChild(picker, "selected-row-fr").isDefault, true)
            verify(findChild(picker, "move-down-fr").activeFocus)
            keyClick(Qt.Key_Space)
            wait(40)
            compare(picker.selectedIds.join(","), "us,fr,il,de")
            mouseClick(findChild(picker, "move-down-il"))
            wait(40)
            compare(picker.selectedIds.join(","), "us,fr,de,il")
            verify(findChild(picker, "move-up-il").activeFocus)
            compare(picker.selectedIds.length, 4)
            picker.selectedIds = ["us"]
            wait(40)
            compare(findChild(picker, "move-up-us").enabled, false)
            compare(findChild(picker, "move-down-us").enabled, false)
        }
        function test_single_list_data() { return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}] }
        function test_drag_reorder_data() {
            const rows = []
            for (const touch of [false, true])
                for (const rtl of [false, true])
                    for (const scale of [1, 0.5])
                        rows.push({tag: (touch ? "touch" : "mouse") + (rtl ? "-RTL" : "-LTR") + "-" + scale,
                                   touch: touch, rtl: rtl, scale: scale})
            return rows
        }
        function dragSelected(id, destination, useTouch, cancel, useGrip) {
            const original = picker.selectedIds.join(",")
            const area = findChild(picker, "drag-" + id)
            const row = findChild(picker, "selected-row-" + picker.selectedIds[destination])
            const gripX = picker.rtl ? area.width - 6 * picker.uiScale : 6 * picker.uiScale
            const start = area.mapToItem(picker, useGrip ? gripX : area.width / 2, area.height / 2)
            const end = Qt.point(start.x, row.mapToItem(picker, row.width / 2, row.height / 2).y)
            const touch = useTouch ? touchEvent(picker) : null
            if (touch) touch.press(0, picker, start.x, start.y).commit()
            else mousePress(picker, start.x, start.y)
            wait(20)
            for (let step = 1; step <= 8; ++step) {
                const x = start.x + (end.x - start.x) * step / 8
                const y = start.y + (end.y - start.y) * step / 8
                if (touch) touch.move(0, picker, x, y).commit()
                else mouseMove(picker, x, y)
                wait(15)
            }
            compare(picker.selectedIds.join(","), original)
            compare(picker.draggedId, id)
            compare(picker.dragDestination, destination)
            verify(findChild(picker, "keyboard-drop-indicator").visible)
            if (cancel) keyClick(Qt.Key_Escape)
            if (touch) touch.release(0, picker, end.x, end.y).commit()
            else mouseRelease(picker, end.x, end.y)
            tryCompare(picker, "draggedId", "")
            wait(40)
        }
        function test_drag_reorder(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(60)
            dragSelected("de", 0, data.touch, false, data.scale < 1)
            compare(picker.selectedIds.join(","), "de,us,il,fr")
            compare(findChild(picker, "selected-row-de").isDefault, true)
            compare(reorderSpy.count, 1)
            dragSelected("de", 3, data.touch, false, data.scale < 1)
            compare(picker.selectedIds.join(","), "us,il,fr,de")
            compare(findChild(picker, "selected-row-us").isDefault, true)
            compare(reorderSpy.count, 2)
            dragSelected("fr", 0, data.touch, true)
            compare(picker.selectedIds.join(","), "us,il,fr,de")
            compare(reorderSpy.count, 2)
            mouseClick(findChild(picker, "move-up-de"))
            wait(40)
            compare(picker.selectedIds.join(","), "us,il,de,fr")
        }
        function test_drag_taps_single_layout_and_outside_drop() {
            const area = findChild(picker, "drag-il")
            mouseClick(area)
            const touch = touchEvent(area)
            touch.press(0, area, area.width / 2, area.height / 2).commit()
            touch.release(0, area, area.width / 2, area.height / 2).commit()
            wait(50)
            compare(reorderSpy.count, 0)
            compare(picker.selectedIds.join(","), "us,il")
            const start = area.mapToItem(picker, area.width / 2, area.height / 2)
            mousePress(picker, start.x, start.y)
            mouseMove(picker, start.x, start.y - 25)
            wait(30)
            mouseMove(picker, -100, start.y - 25)
            wait(30)
            compare(picker.dragDestination, -1)
            mouseRelease(picker, -100, start.y - 25)
            wait(50)
            compare(reorderSpy.count, 0)
            picker.selectedIds = ["us"]
            wait(40)
            const single = findChild(picker, "drag-us")
            mouseDrag(single, single.width / 2, single.height / 2, 0, 80)
            wait(40)
            compare(picker.draggedId, "")
            compare(picker.selectedIds.join(","), "us")
        }
        function test_touch_controls_remain_independent_data() { return test_scroll_data() }
        function test_touch_controls_remain_independent(data) {
            picker.rtl = data.rtl
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(40)
            function tap(name) {
                const control = findChild(picker, name)
                const touch = touchEvent(control)
                touch.press(0, control, control.width / 2, control.height / 2).commit()
                touch.release(0, control, control.width / 2, control.height / 2).commit()
                wait(40)
            }
            tap("move-up-fr")
            compare(picker.selectedIds.join(","), "us,fr,il,de")
            tap("move-up-il")
            tap("move-up-il")
            compare(picker.selectedIds.join(","), "il,us,fr,de")
            tap("remove-de")
            compare(picker.selectedIds.join(","), "il,us,fr")
            compare(picker.draggedId, "")
            compare(reorderSpy.count, 0)
        }
        function test_drag_wrapped_names_data() {
            return [{tag: "mouse-LTR", touch: false, rtl: false}, {tag: "touch-RTL", touch: true, rtl: true}]
        }
        function test_drag_wrapped_names(data) {
            testSurface.width = 380
            testSurface.height = 900
            picker.rtl = data.rtl
            picker.layouts = originalLayouts.map((layout, index) => ({id: layout.id,
                name: layout.name + " " + "Long translated keyboard arrangement description ".repeat(index + 1)}))
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(60)
            verify(findChild(picker, "selected-row-de").height > findChild(picker, "selected-row-us").height)
            dragSelected("de", 0, data.touch, false)
            compare(picker.selectedIds.join(","), "de,us,il,fr")
            compare(findChild(picker, "selected-row-de").isDefault, true)
            compare(reorderSpy.count, 1)
        }
        function test_drag_interrupted_data() {
            return ["hidden", "disabled", "resize", "model", "direction"].map(reason => ({tag: reason, reason: reason}))
        }
        function test_default_label_without_radio_controls_data() { return test_scroll_data() }
        function test_action_icons_and_grip_inset_data() { return test_drag_reorder_data().filter(row => !row.touch) }
        function test_action_icons_and_grip_inset(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            picker.selectedIds = ["us", "il"]
            wait(50)
            const row = findChild(picker, "selected-row-us")
            const area = findChild(picker, "drag-us")
            const point = area.mapToItem(row, picker.rtl ? area.width : 0, 0)
            const inset = picker.rtl ? row.width - point.x : point.x
            verify(Math.abs(inset - 12 * data.scale) < 1)
            const trash = findChild(picker, "remove-icon-il")
            tryCompare(trash, "status", Image.Ready)
            verify(trash.source.toString().endsWith("assets/edit-delete.svg"))
            compare(trash.opacity, 1)
            picker.searchText = "French"
            wait(50)
            const plus = findChild(picker, "add-icon-fr")
            verify(plus)
            compare(plus.color.toString(), "#16823e")
            picker.selectedIds = ["us", "il", "de", "gb"]
            wait(50)
            compare(findChild(picker, "add-icon-fr").color.toString(), "#81848a")
            picker.selectedIds = ["us"]
            wait(50)
            compare(findChild(picker, "remove-us").enabled, false)
            compare(findChild(picker, "remove-icon-us").opacity, 0.4)
        }
        function test_default_label_without_radio_controls(data) {
            picker.rtl = data.rtl
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(40)
            for (const id of picker.selectedIds) {
                compare(findChild(picker, "default-" + id), null)
                compare(findChild(picker, "selected-row-" + id).isDefault, id === "us")
                compare(findChild(picker, "selected-name-" + id).text.endsWith(" - Default"), id === "us")
            }
            picker.selectedIds = ["il"]
            wait(40)
            compare(findChild(picker, "selected-row-il").isDefault, true)
            verify(findChild(picker, "selected-name-il").text.endsWith(" - Default"))
        }
        function test_drag_interrupted(data) {
            picker.selectedIds = ["us", "il", "fr"]
            wait(40)
            const area = findChild(picker, "drag-fr")
            const first = findChild(picker, "selected-row-us")
            const start = area.mapToItem(picker, area.width / 2, area.height / 2)
            const end = first.mapToItem(picker, first.width / 2, first.height / 2)
            const touch = touchEvent(picker)
            touch.press(0, picker, start.x, start.y).commit()
            for (let step = 1; step <= 8; ++step) {
                touch.move(0, picker, start.x, start.y + (end.y - start.y) * step / 8).commit()
                wait(15)
            }
            compare(picker.draggedId, "fr")
            if (data.reason === "hidden") picker.visible = false
            else if (data.reason === "disabled") picker.enabled = false
            else if (data.reason === "resize") testSurface.width = 550
            else if (data.reason === "model") picker.layouts = picker.layouts.map(layout => ({id: layout.id, name: layout.name + " updated"}))
            else picker.rtl = true
            tryCompare(picker, "draggedId", "")
            touch.release(0, picker, end.x, end.y).commit()
            wait(60)
            compare(picker.selectedIds.join(","), "us,il,fr")
            compare(reorderSpy.count, 0)
        }
        function test_single_list(data) {
            picker.rtl = data.rtl
            picker.layouts = [
                {id: "fr", name: "French (AZERTY)"},
                {id: "us", name: "U.S. English (QWERTY)"},
                {id: "de", name: "German (QWERTZ)"},
                {id: "us:dvorak", name: "English (Dvorak)"}
            ]
            picker.selectedIds = ["fr"]
            wait(60)
            compare(picker.filteredLayouts.map(layout => layout.id).join(","), "us,de,us:dvorak")
            compare(picker.resultsView.count, 3)
            compare(findChild(picker, "keyboard-section-other").text, "Other layouts")
            compare(findChild(picker, "keyboard-section-suggested"), null)
            compare(picker.selectedIds.join(","), "fr")
            picker.resultsView.forceActiveFocus()
            keyClick(Qt.Key_Home)
            keyClick(Qt.Key_Down)
            compare(picker.filteredLayouts[picker.resultsView.currentIndex].id, "de")
            keyClick(Qt.Key_Up)
            compare(picker.filteredLayouts[picker.resultsView.currentIndex].id, "us")
            picker.searchText = "QWERTZ"
            wait(60)
            compare(picker.filteredLayouts.length, 1)
            compare(picker.selectedIds.join(","), "fr")
            picker.focusSearch()
            keyClick(Qt.Key_Enter)
            compare(picker.selectedIds.join(","), "fr,de")
        }
        function test_small_portrait_layout() {
            picker.uiScale = 0.5
            picker.rtl = true
            picker.selectedIds = ["us", "il", "fr", "de"]
            wait(30)
            verify(picker.resultsView.height >= 30)
            verify(picker.searchInput.width > 0)
            compare(picker.selectedLayouts.length, 4)
        }
        function scrollSetup(rtl) {
            picker.rtl = rtl
            picker.layouts = originalLayouts.concat(Array.from({length: 50}, (_, index) => ({id: "extra" + index, name: "Extra layout " + index, search: ""})))
            picker.resultsView.forceActiveFocus()
            picker.resultsView.positionViewAtBeginning()
            wait(100)
        }
        function test_scroll_data() { return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}] }
        function test_scroll(data) {
            scrollSetup(data.rtl)
            const view = picker.resultsView
            const start = view.contentY
            mouseWheel(view, 100, 100, 0, -240)
            tryVerify(function() { return view.contentY > start + 10 })
            wait(400)
            const scrolled = view.contentY
            const originalX = view.contentX
            mouseWheel(view, 100, 100, -120, 0)
            wait(400)
            compare(view.contentX, originalX)
            verify(Math.abs(view.contentY - scrolled) < 1)
            for (let i = 0; i < 12; ++i) mouseWheel(view, 100, 100, 0, -1200)
            wait(700)
            verify(view.atYEnd)
            const bottom = view.contentY
            mouseWheel(view, 100, 100, 0, -1200)
            wait(400)
            verify(Math.abs(view.contentY - bottom) < 1)
            keyClick(Qt.Key_Home)
            tryVerify(function() { return view.atYBeginning })
        }
        function test_touch_data() { return test_scroll_data() }
        function test_touch(data) {
            scrollSetup(data.rtl)
            const view = picker.resultsView
            const start = view.contentY
            const touch = touchEvent(view)
            touch.press(0, view, 100, 220).commit()
            wait(20)
            for (let y = 200; y >= 60; y -= 20) {
                touch.move(0, view, 100, y).commit()
                wait(20)
            }
            touch.release(0, view, 100, 60).commit()
            tryVerify(function() { return view.contentY > start + 20 })
            view.cancelFlick()
            compare(picker.selectedIds.join(","), "us,il")
        }
    }
}
