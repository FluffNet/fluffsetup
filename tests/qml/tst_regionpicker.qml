import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    property var originalRegions
    width: 520
    height: 300
    LanguagePicker {
        id: picker
        anchors.fill: parent
        languages: [
            {code: "de", english: "Deutschland", native: "Deutsch (Deutschland)", label: "Deutsch (Deutschland)", cycle: "Deutschland"},
            {code: "de_BE", english: "Belgien", native: "Deutsch (Belgien)", label: "Deutsch (Belgien)", cycle: "Belgien"},
            {code: "de_CH", english: "Schweiz", native: "Deutsch (Schweiz)", label: "Deutsch (Schweiz)", cycle: "Schweiz"},
            {code: "de_AT", english: "Österreich", native: "Deutsch (Österreich)", label: "Deutsch (Österreich)", cycle: "Österreich"}
        ]
        selectedCode: "de"
        onLanguageSelected: code => selectedCode = code
    }
    SignalSpy { id: spy; target: picker; signalName: "continueRequested" }
    TestCase {
        name: "RegionPicker"
        when: windowShown
        function initTestCase() { originalRegions = picker.languages }
        function init() {
            picker.cancelFlick()
            picker.languages = originalRegions
            picker.selectedCode = "de"
            picker.rtl = false
            picker.forceActiveFocus()
            spy.clear()
            wait(100)
        }
        function test_country_initial_not_language_initial() {
            keyClick(Qt.Key_B)
            compare(picker.selectedCode, "de_BE")
            keyClick(Qt.Key_O)
            compare(picker.selectedCode, "de_AT")
            keyClick(Qt.Key_D)
            compare(picker.selectedCode, "de")
            keyClick(Qt.Key_S)
            compare(picker.selectedCode, "de_CH")
        }
        function test_no_english_duplicate() {
            const row = findChild(picker, "language-de_AT")
            verify(row)
            compare(row.contentItem.text, "\u2068Deutsch (Österreich)\u2069")
        }
        function test_touch_scroll_and_tap_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_touch_scroll_and_tap(data) {
            picker.rtl = data.rtl
            picker.languages = originalRegions.concat(Array.from({length: 24}, (_, index) =>
                ({code: "test-" + index, english: "Test country " + index,
                  native: "Test country " + index, label: "Test country " + index})))
            wait(100)
            picker.positionViewAtBeginning()
            const start = picker.contentY
            const touch = touchEvent(picker)
            touch.press(0, picker, 200, 240).commit()
            wait(20)
            for (let y = 220; y >= 60; y -= 20) {
                touch.move(0, picker, 200, y).commit()
                wait(20)
            }
            touch.release(0, picker, 200, 60).commit()
            tryVerify(function() { return picker.contentY > start + 20 })
            picker.cancelFlick()
            compare(picker.selectedCode, "de")
            const scrolled = picker.contentY
            mouseWheel(picker, 200, 120, 0, 240)
            tryVerify(function() { return picker.contentY < scrolled })
            picker.cancelFlick()
            picker.positionViewAtBeginning()
            wait(100)
            const row = picker.itemAtIndex(1)
            verify(row)
            touch.press(0, row, row.width / 2, row.height / 2).commit()
            touch.release(0, row, row.width / 2, row.height / 2).commit()
            tryCompare(picker, "selectedCode", "de_BE")
        }
        function test_arrows_and_both_enter_data() {
            return [{tag: "regular-ltr", key: Qt.Key_Return, rtl: false},
                    {tag: "numpad-ltr", key: Qt.Key_Enter, rtl: false},
                    {tag: "regular-rtl", key: Qt.Key_Return, rtl: true},
                    {tag: "numpad-rtl", key: Qt.Key_Enter, rtl: true}]
        }
        function test_arrows_and_both_enter(data) {
            picker.rtl = data.rtl
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "de_BE")
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "de_CH")
            keyClick(Qt.Key_Up)
            compare(picker.selectedCode, "de_BE")
            keyClick(data.key)
            compare(spy.count, 1)
            compare(picker.selectedCode, "de_BE")
            compare(picker.rtl, data.rtl)
        }
    }
}
