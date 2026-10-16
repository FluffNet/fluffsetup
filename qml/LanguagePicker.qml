import QtQuick
import QtQuick.Controls
import org.kde.kirigami as Kirigami

ListView {
    id: picker
    property var languages: []
    property string selectedCode: "en"
    property bool rtl: false
    property real uiScale: 1
    property bool grouped: false
    property string labelLanguage: ""
    property bool allowUnselectedContinue: false
    readonly property int selectedIndex: languages.findIndex(language => language.code === selectedCode)
    readonly property bool hasSelection: selectedIndex >= 0
    function cycleKey(value) {
        return value.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "")
    }
    function nameFont(code) {
        if (Qt.platform.os !== "osx") return Qt.application.font.family
        const base = code.split("_")[0]
        if (["hi", "mr", "ne", "kok"].includes(base)) return "Devanagari Sangam MN"
        if (base === "gu") return "Gujarati Sangam MN"
        if (base === "ta") return "Tamil Sangam MN"
        if (base === "te") return "Telugu Sangam MN"
        if (["as", "bn"].includes(base)) return "Bangla Sangam MN"
        if (base === "or") return "Oriya Sangam MN"
        if (base === "kn") return "Kannada Sangam MN"
        if (base === "ml") return "Malayalam Sangam MN"
        if (base === "pa" && code !== "pa_Arab" && !code.startsWith("pa_PK")) return "Gurmukhi Sangam MN"
        if (base === "si") return "Sinhala Sangam MN"
        if (base === "km") return "Khmer Sangam MN"
        if (base === "lo") return "Lao Sangam MN"
        if (base === "my") return "Myanmar Sangam MN"
        return Qt.application.font.family
    }
    signal languageSelected(string code)
    signal continueRequested()

    model: languages
    section.property: grouped ? "group" : ""
    section.criteria: ViewSection.FullString
    section.delegate: Text {
        required property string section
        objectName: "picker-section-" + section
        width: picker.width - picker.leftMargin - picker.rightMargin
        height: implicitHeight + Math.round(12 * picker.uiScale)
        text: section
        color: "#55565a"
        font.pixelSize: Math.round(15 * picker.uiScale)
        horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
        verticalAlignment: Text.AlignVCenter
        wrapMode: Text.Wrap
        LayoutMirroring.enabled: false
    }
    clip: true
    // Native touchscreen dragging/flicking coexists with Kirigami wheel input.
    interactive: true
    activeFocusOnTab: true
    boundsBehavior: Flickable.StopAtBounds
    flickableDirection: Flickable.VerticalFlick
    keyNavigationEnabled: false
    spacing: Math.round(4 * uiScale)
    leftMargin: rtl ? scrollBar.width + Math.round(8 * uiScale) : 0
    rightMargin: rtl ? 0 : scrollBar.width + Math.round(8 * uiScale)
    LayoutMirroring.enabled: false
    LayoutMirroring.childrenInherit: true

    function syncSelection() {
        const index = selectedIndex
        currentIndex = index
        if (index < 0) return
        Qt.callLater(function() {
            forceLayout()
            currentIndex = selectedIndex
            if (currentIndex < 0) return
            const priorityRowFits = currentIndex < 9 && currentItem
                    && currentItem.y - originY + currentItem.height <= height
            // Reveal the selected row even when a small viewport cannot fit
            // all the priority languages. Home still returns to the top.
            if (priorityRowFits) positionViewAtBeginning()
            else positionViewAtIndex(currentIndex, ListView.Contain)
        })
    }
    function choose(index) {
        if (index < 0 || index >= count) return
        languageSelected(languages[index].code)
        // Also reveal a selection that did not change, e.g. Home after wheel
        // scrolling away from the still-selected English row.
        syncSelection()
    }
    function cycleLetter(letter) {
        const matches = []
        for (let i = 0; i < languages.length; ++i)
            if (cycleKey(languages[i].cycle || languages[i].english).startsWith(cycleKey(letter))
                    || (grouped && cycleKey(languages[i].english).startsWith(cycleKey(letter)))) matches.push(i)
        if (matches.length === 0) return
        const current = matches.indexOf(currentIndex)
        choose(matches[(current + 1) % matches.length])
    }
    onSelectedCodeChanged: syncSelection()
    onLanguagesChanged: syncSelection()
    onCountChanged: syncSelection()
    Component.onCompleted: syncSelection()

    Keys.onPressed: event => {
        switch (event.key) {
        case Qt.Key_Up: choose(Math.max(0, currentIndex - 1)); break
        case Qt.Key_Down: choose(Math.min(count - 1, currentIndex + 1)); break
        case Qt.Key_Home: choose(0); break
        case Qt.Key_End: choose(count - 1); break
        case Qt.Key_PageUp: choose(Math.max(0, currentIndex - 8)); break
        case Qt.Key_PageDown: choose(Math.min(count - 1, currentIndex + 8)); break
        case Qt.Key_Return:
        case Qt.Key_Enter: if (hasSelection || allowUnselectedContinue) continueRequested(); break
        default:
            if (event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return
            if (event.key >= Qt.Key_A && event.key <= Qt.Key_Z)
                cycleLetter(String.fromCharCode(event.key).toLowerCase())
            else if (event.text.length === 1 && !/[\s0-9]/.test(event.text))
                cycleLetter(event.text.toLowerCase())
            else return
        }
        event.accepted = true
    }

    delegate: ItemDelegate {
        id: languageRow
        required property int index
        required property var modelData
        objectName: "language-" + modelData.code
        readonly property bool selected: modelData.code === picker.selectedCode
        width: picker.width - picker.leftMargin - picker.rightMargin
        height: Math.max(Math.round(44 * picker.uiScale), rowText.implicitHeight + Math.round(20 * picker.uiScale))
        padding: Math.round(10 * picker.uiScale)
        focusPolicy: Qt.NoFocus
        onClicked: {
            picker.choose(index)
            picker.forceActiveFocus()
        }
        // Isolate names independently so mixed scripts keep their English-first
        // order. This never reverses Latin letters within a right-aligned row.
        contentItem: Text {
            id: rowText
            text: languageRow.modelData.label !== undefined ? "\u2068" + languageRow.modelData.label + "\u2069"
                  : "\u2068" + languageRow.modelData.english + "\u2069"
                  + (languageRow.modelData.english === languageRow.modelData.native ? ""
                     : "  \u2068" + languageRow.modelData.native + "\u2069")
            textFormat: Text.PlainText
            color: languageRow.selected ? "white" : "#111111"
            font.pixelSize: Math.max(1, Math.round(17 * picker.uiScale))
            // Use macOS's shaping-capable script fonts in development previews.
            // Linux continues to use its installed system font fallback.
            font.family: picker.nameFont(picker.labelLanguage || languageRow.modelData.code)
            wrapMode: Text.Wrap
            horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: Math.round(6 * picker.uiScale)
            color: languageRow.selected ? "#820101" : languageRow.hovered ? "#e7e9ed" : "#ffffff"
            border.width: languageRow.selected && picker.activeFocus ? 2 : 1
            border.color: languageRow.selected && picker.activeFocus ? "#111111" : "#d6d8dc"
        }
        Accessible.name: modelData.label || modelData.english + " " + modelData.native
    }

    Item {
        id: scrollBarContainer
        anchors.fill: parent
        z: 2
    }
    ScrollBar.vertical: ScrollBar {
        id: scrollBar
        objectName: "languageScrollBar"
        // A separate parent disables the attached control's automatic geometry
        // so switching RTL/LTR cannot overwrite the left/right placement.
        parent: scrollBarContainer
        policy: ScrollBar.AlwaysOn
        width: Math.max(8, Math.round(12 * picker.uiScale))
        x: picker.rtl ? 0 : picker.width - width
        anchors.top: parent.top
        anchors.bottom: parent.bottom
    }
    Kirigami.WheelHandler {
        target: picker
        blockTargetWheel: true
        scrollFlickableTarget: true
        filterMouseEvents: false
    }
}
