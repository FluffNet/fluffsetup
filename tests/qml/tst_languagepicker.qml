import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    width: 520
    height: 480
    LanguagePicker {
        id: picker
        anchors.fill: parent
        selectedCode: "en"
        rtl: ["he", "ar", "fa", "ur"].includes(selectedCode)
        languages: [
            {code: "en", english: "English", native: "English"},
            {code: "es", english: "Spanish", native: "Español"},
            {code: "fr", english: "French", native: "Français"},
            {code: "pt", english: "Portuguese", native: "Português"},
            {code: "uk", english: "Ukrainian", native: "Українська"},
            {code: "he", english: "Hebrew", native: "עברית"},
            {code: "ru", english: "Russian", native: "Русский"},
            {code: "ar", english: "Arabic", native: "العربية"},
            {code: "de", english: "German", native: "Deutsch"},
            {code: "bn", english: "Bengali", native: "বাংলা"},
            {code: "bg", english: "Bulgarian", native: "Български"},
            {code: "hi", english: "Hindi", native: "हिन्दी"},
            {code: "hu", english: "Hungarian", native: "Magyar"},
            {code: "ja", english: "Japanese", native: "日本語"},
            {code: "fa", english: "Persian", native: "فارسی"},
            {code: "ur", english: "Urdu", native: "اردو"},
            {code: "vi", english: "Vietnamese", native: "Tiếng Việt"}
        ]
        onLanguageSelected: code => selectedCode = code
    }
    SignalSpy { id: continueSpy; target: picker; signalName: "continueRequested" }
    TestCase {
        name: "LanguagePicker"
        when: windowShown
        function test_missing_selection_cannot_continue_with_either_enter_key() {
            for (let code of ["", "unknown", "en_US"]) {
                picker.selectedCode = code
                wait(30)
                compare(picker.hasSelection, false)
                compare(picker.currentIndex, -1)
                keyClick(Qt.Key_Return)
                keyClick(Qt.Key_Enter)
                compare(continueSpy.count, 0)
            }
            picker.choose(0)
            compare(picker.hasSelection, true)
            keyClick(Qt.Key_Return)
            compare(continueSpy.count, 1)
        }
        function init() {
            picker.cancelFlick()
            picker.selectedCode = "en"
            picker.width = 520
            picker.height = 480
            picker.uiScale = 1
            picker.syncSelection()
            picker.forceActiveFocus()
            continueSpy.clear()
            wait(100)
        }
        function test_arrows_and_live_direction() {
            compare(picker.currentIndex, 0)
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "es")
            compare(picker.rtl, false)
            keyClick(Qt.Key_H)
            compare(picker.selectedCode, "he")
            compare(picker.rtl, true)
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "ru")
            compare(picker.rtl, false)
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "ar")
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "de")
            compare(picker.rtl, false)
            keyClick(Qt.Key_Up)
            compare(picker.selectedCode, "ar")
            keyClick(Qt.Key_Home)
            compare(picker.selectedCode, "en")
            keyClick(Qt.Key_Up)
            compare(picker.selectedCode, "en")
            keyClick(Qt.Key_End)
            compare(picker.selectedCode, "vi")
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "vi")
        }
        function test_letter_cycling_and_modifiers() {
            for (let code of ["he", "hi", "hu", "he"]) {
                keyClick(Qt.Key_H)
                compare(picker.selectedCode, code)
            }
            keyClick(Qt.Key_G, Qt.ControlModifier)
            compare(picker.selectedCode, "he")
            keyClick(Qt.Key_Z)
            compare(picker.selectedCode, "he")
            keyClick(Qt.Key_B)
            compare(picker.selectedCode, "bn")
            keyClick(Qt.Key_B, Qt.ShiftModifier)
            compare(picker.selectedCode, "bg")
            keyClick(Qt.Key_P)
            compare(picker.selectedCode, "pt")
            keyClick(Qt.Key_P)
            compare(picker.selectedCode, "fa")
        }
        function test_enter_data() {
            return [{tag: "regular", key: Qt.Key_Return, modifier: Qt.NoModifier},
                    {tag: "numpad", key: Qt.Key_Enter, modifier: Qt.KeypadModifier}]
        }
        function test_enter(data) {
            keyClick(Qt.Key_H)
            keyClick(data.key, data.modifier)
            compare(continueSpy.count, 1)
            compare(picker.selectedCode, "he")
        }
        function test_pointer_selects_and_preserves_keyboard() {
            const row = findChild(picker, "language-he")
            verify(row)
            mouseClick(row, row.width / 2, row.height / 2)
            compare(picker.selectedCode, "he")
            compare(picker.activeFocus, true)
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "ru")
        }
        function test_scroll_data() {
            return [{tag: "ltr", code: "en"}, {tag: "rtl", code: "he"}]
        }
        function test_scroll(data) {
            picker.selectedCode = data.code
            wait(100)
            const start = picker.contentY
            mouseWheel(picker, 200, 200, 0, -240)
            tryVerify(function() { return picker.contentY > start + 10 })
            wait(400)
            const scrolled = picker.contentY
            const originalX = picker.contentX
            // The vertical-only list must not accidentally pan sideways.
            mouseWheel(picker, 200, 200, -120, 0)
            wait(400)
            compare(picker.contentX, originalX)
            verify(Math.abs(picker.contentY - scrolled) < 1)
            for (let i = 0; i < 6; ++i) mouseWheel(picker, 200, 200, 0, -1200)
            wait(700)
            verify(picker.atYEnd)
            const bottom = picker.contentY
            mouseWheel(picker, 200, 200, 0, -1200)
            wait(400)
            verify(Math.abs(picker.contentY - bottom) < 1)
            keyClick(Qt.Key_Home)
            tryVerify(function() { return picker.atYBeginning })
            wait(400)
            const top = picker.contentY
            mouseWheel(picker, 200, 200, 0, 1200)
            wait(400)
            verify(Math.abs(picker.contentY - top) < 1)
        }
        function test_scrollbar_data() { return test_scroll_data() }
        function test_scrollbar(data) {
            picker.selectedCode = data.code
            wait(100)
            const bar = findChild(picker, "languageScrollBar")
            verify(bar)
            compare(bar.visible, true)
            verify(data.code === "he" ? bar.x === 0 : bar.x + bar.width === picker.width)
            mouseDrag(bar, bar.width / 2, 30, 0, 150)
            tryVerify(function() { return picker.contentY > 20 })
        }
        function test_touch_data() { return test_scroll_data() }
        function test_touch(data) {
            picker.selectedCode = data.code
            wait(100)
            const start = picker.contentY
            const touch = touchEvent(picker)
            touch.press(0, picker, 200, 350).commit()
            wait(20)
            for (let y = 330; y >= 120; y -= 30) {
                touch.move(0, picker, 200, y).commit()
                wait(20)
            }
            touch.release(0, picker, 200, 120).commit()
            tryVerify(function() { return picker.contentY > start + 20 })
            picker.cancelFlick()
            compare(picker.selectedCode, data.code)
        }
        function test_recovered_selection_visible_at_small_size_data() {
            return [{tag: "priority", code: "de", index: 8},
                    {tag: "rtl", code: "ur", index: 15}]
        }
        function test_priority_group_stays_at_top_when_it_fits() {
            for (let code of ["en", "es", "fr", "pt", "uk", "he", "ru", "ar", "de"]) {
                picker.selectedCode = code
                wait(100)
                verify(picker.atYBeginning)
                for (let index = 0; index < 9; ++index) {
                    const row = picker.itemAtIndex(index)
                    verify(row && row.visible)
                    const position = row.mapToItem(picker, 0, 0)
                    verify(position.y >= -1)
                    verify(position.y + row.height <= picker.height + 1)
                }
            }
        }
        function test_recovered_selection_visible_at_small_size(data) {
            picker.width = 330
            picker.height = 200
            picker.uiScale = 0.7
            picker.selectedCode = data.code
            wait(100)
            compare(picker.currentIndex, data.index)
            const row = picker.currentItem
            const position = row.mapToItem(picker, 0, 0)
            verify(position.y >= -1)
            verify(position.y + row.height <= picker.height + 1)
            compare(picker.rtl, data.code === "ur")
        }
    }
}
