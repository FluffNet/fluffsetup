import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    width: 520
    height: 340
    LanguagePicker {
        id: picker
        anchors.fill: parent
        grouped: true
        selectedCode: "IL"
        languages: [
            {code: "IL", english: "Israel", label: "ישראל", cycle: "ישראל", group: "Suggested regions"},
            {code: "DE", english: "Germany", label: "גרמניה", cycle: "גרמניה", group: "Other regions"},
            {code: "HU", english: "Hungary", label: "הונגריה", cycle: "הונגריה", group: "Other regions"},
            {code: "IN", english: "India", label: "הודו", cycle: "הודו", group: "Other regions"},
            {code: "IE", english: "Ireland", label: "אירלנד", cycle: "אירלנד", group: "Other regions"},
            {code: "IT", english: "Italy", label: "איטליה", cycle: "איטליה", group: "Other regions"},
            {code: "JP", english: "Japan", label: "יפן", cycle: "יפן", group: "Other regions"},
            {code: "KR", english: "South Korea", label: "קוריאה הדרומית", cycle: "קוריאה הדרומית", group: "Other regions"},
            {code: "US", english: "United States", label: "ארצות הברית", cycle: "ארצות הברית", group: "Other regions"}
        ]
        onLanguageSelected: code => selectedCode = code
    }
    SignalSpy { id: nextSpy; target: picker; signalName: "continueRequested" }
    TestCase {
        name: "CountryPicker"
        when: windowShown
        function init() {
            picker.cancelFlick()
            picker.selectedCode = "IL"
            picker.rtl = false
            picker.forceActiveFocus()
            picker.syncSelection()
            nextSpy.clear()
            wait(100)
        }
        function test_group_headers_and_no_search() {
            const suggested = findChild(picker, "picker-section-Suggested regions")
            const other = findChild(picker, "picker-section-Other regions")
            verify(suggested && other)
            compare(suggested.text, "Suggested regions")
            compare(other.text, "Other regions")
            verify(suggested.mapToItem(picker, 0, 0).y < picker.currentItem.mapToItem(picker, 0, 0).y)
            verify(picker.currentItem.contentItem.text.includes("ישראל"))
            verify(!picker.currentItem.contentItem.text.includes("Israel"))
            compare(findChild(picker, "keyboard-search"), null)
        }
        function test_navigation_data() {
            return [{tag: "LTR-return", rtl: false, key: Qt.Key_Return},
                    {tag: "RTL-enter", rtl: true, key: Qt.Key_Enter}]
        }
        function test_navigation(data) {
            picker.rtl = data.rtl
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "DE")
            keyClick(Qt.Key_Up)
            compare(picker.selectedCode, "IL")
            // Country names remain localized; ASCII initials work before the
            // user has chosen a keyboard, in addition to localized initials.
            keyClick(Qt.Key_I)
            compare(picker.selectedCode, "IN")
            keyClick(Qt.Key_I)
            compare(picker.selectedCode, "IE")
            keyClick(Qt.Key_End)
            compare(picker.selectedCode, "US")
            keyClick(Qt.Key_Home)
            compare(picker.selectedCode, "IL")
            keyClick(data.key)
            compare(nextSpy.count, 1)
            compare(picker.rtl, data.rtl)
        }
        function test_scroll_and_touch_data() {
            return [{tag: "LTR", rtl: false}, {tag: "RTL", rtl: true}]
        }
        function test_scroll_and_touch(data) {
            picker.rtl = data.rtl
            mouseWheel(picker, 250, 140, 0, -240)
            tryVerify(function() { return picker.contentY > picker.originY })
            compare(picker.selectedCode, "IL")
            // Kirigami's wheel animation is distinct from native flicking.
            // Let the preceding vertical wheel gesture settle before testing
            // that a horizontal gesture cannot move this vertical-only list.
            wait(400)
            picker.cancelFlick()
            const originalX = picker.contentX
            const originalY = picker.contentY
            mouseWheel(picker, 250, 140, -120, 0)
            wait(100)
            compare(picker.contentX, originalX)
            verify(Math.abs(picker.contentY - originalY) < 1)
            picker.positionViewAtBeginning()
            const touch = touchEvent(picker)
            touch.press(0, picker, 260, 250).commit()
            wait(30)
            touch.move(0, picker, 260, 180).commit()
            wait(30)
            touch.move(0, picker, 260, 80).commit()
            touch.release(0, picker, 260, 80).commit()
            tryVerify(function() { return picker.contentY > picker.originY })
            compare(picker.selectedCode, "IL")
            picker.cancelFlick()
            const scrolled = picker.contentY
            mouseWheel(picker, 250, 140, 0, 240)
            tryVerify(function() { return picker.contentY < scrolled })
            picker.cancelFlick()
            picker.positionViewAtBeginning()
            wait(100)
            const row = picker.itemAtIndex(1)
            touch.press(0, row, row.width / 2, row.height / 2).commit()
            touch.release(0, row, row.width / 2, row.height / 2).commit()
            tryCompare(picker, "selectedCode", "DE")
        }
    }
}
