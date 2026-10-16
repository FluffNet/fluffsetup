import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property var zones: []
    property string selectedCode: "UTC"
    property string language: "en"
    property bool rtl: false
    property real uiScale: 1
    property bool mapEnabled: Qt.platform.os !== "osx"
    property bool mapRequested: false
    property string searchPlaceholder: "Search by city or country"
    property string emptyText: "No matching time zones"
    property string mapUnavailableText: "Map unavailable. Use the search below."
    property var mapText: (context, source) => source
    property alias searchInput: search
    property alias searchText: search.text
    property alias resultView: results
    readonly property var mapItem: mapLoader.item
    readonly property var selectedCenter: {
        const selected = zones.find(zone => zone.code === selectedCode)
        if (selected && selected.center) return selected.center
        if (selectedCode.length) return null
        const suggested = zones.filter(zone => zone.suggested && zone.center)
        if (!suggested.length) return null
        return [suggested.reduce((sum, zone) => sum + zone.center[0], 0) / suggested.length,
                suggested.reduce((sum, zone) => sum + zone.center[1], 0) / suggested.length]
    }
    readonly property real minimumResultsHeight: 5 * Math.round(44 * uiScale) + 4 * Math.round(4 * uiScale)
    readonly property string selectedLabel: {
        const row = zones.find(zone => zone.code === selectedCode)
        return row ? row.label : selectedCode.length ? selectedCode.replace(/_/g, " ") : root.mapText("Placeholder for empty time zone combobox selector", "Choose…")
    }
    readonly property var filteredZones: {
        const tokens = searchKey(search.text).trim().split(/\s+/).filter(token => token.length)
        if (!tokens.length) {
            const suggested = zones.filter(zone => zone.suggested)
            if (!suggested.length) return zones
            const selected = zones.find(zone => zone.code === selectedCode)
            return selected && !selected.suggested ? [selected, ...suggested] : suggested
        }
        return zones.filter(zone => tokens.every(token => searchKey(zone.search || zone.label + " " + zone.code).includes(token)))
    }
    readonly property string confirmationCode: {
        if (search.inputMethodComposing) return ""
        if (!search.text.trim().length) return selectedCode
        const matches = filteredZones
        if (!matches.length) return ""
        const selected = matches.find(zone => zone.code === selectedCode)
        return selected ? selected.code : matches[0].code
    }
    readonly property bool canConfirm: confirmationCode.length > 0
    signal zoneSelected(string id)
    signal continueRequested()
    signal typingFocused()
    function searchKey(text) { return String(text).toLowerCase().normalize("NFD").replace(/[\u0300-\u036f\u0591-\u05bd\u05bf\u05c1-\u05c7\u064b-\u065f]/g, "") }
    function focusSearch() { search.forceActiveFocus() }
    function confirm() {
        const code = confirmationCode
        if (!code.length) return
        if (code !== selectedCode || search.text.trim().length) root.zoneSelected(code)
        // Do not continue if the owner/backend rejected this selection.
        if (root.selectedCode === code) root.continueRequested()
    }
    // Keep the native map after its first visit. Destroying its delegates while
    // a selection changes races KDE's bindings on rapid Next/Back navigation.
    function requestVisibleMap() {
        // StackLayout establishes its initial visible page after its children
        // complete. Do not create a hidden native map during that transition.
        Qt.callLater(() => { if (root.visible && root.mapEnabled) root.mapRequested = true })
    }
    onVisibleChanged: requestVisibleMap()
    onMapEnabledChanged: requestVisibleMap()
    Component.onCompleted: requestVisibleMap()
    ColumnLayout {
        anchors.fill: parent
        spacing: Math.round(12 * root.uiScale)
        Rectangle {
            id: mapFrame
            Layout.fillWidth: true
            Layout.preferredHeight: Math.max(0, Math.min(Math.round(root.height * 0.55),
                root.height - root.minimumResultsHeight - search.height - selectedText.implicitHeight - 3 * Math.round(12 * root.uiScale)))
            clip: true
            color: "#edf0f3"
            border.color: "#b9bcc2"
            radius: Math.round(6 * root.uiScale)
            Loader {
                id: mapLoader
                width: parent.width / root.uiScale
                height: parent.height / root.uiScale
                scale: root.uiScale
                transformOrigin: Item.TopLeft
                active: root.mapRequested && root.mapEnabled
                function loadMap() {
                    if (!active || source.toString().length) return
                    setSource(Qt.resolvedUrl("KdeTimezoneMap.qml"), {
                        zone: Qt.binding(() => root.selectedCode),
                        language: Qt.binding(() => root.language),
                        center: Qt.binding(() => root.selectedCenter),
                        preferredZoom: Qt.binding(() => root.selectedCode.length ? 4 : 2),
                        mapText: Qt.binding(() => root.mapText)
                    })
                }
                onActiveChanged: loadMap()
                Component.onCompleted: loadMap()
                onLoaded: {
                    item.zoneSelected.connect(id => { root.searchText = ""; root.zoneSelected(id) })
                }
            }
            Text {
                anchors.fill: parent
                anchors.margins: Math.round(18 * root.uiScale)
                visible: !root.mapEnabled || mapLoader.status === Loader.Error
                text: root.mapUnavailableText
                wrapMode: Text.Wrap
                font.pixelSize: Math.round(16 * root.uiScale)
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                color: "#55565a"
            }
        }
        Text {
            id: selectedText
            Layout.fillWidth: true
            text: root.selectedLabel
            font.pixelSize: Math.round(15 * root.uiScale)
            color: "#111111"
            wrapMode: Text.Wrap
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
            LayoutMirroring.enabled: false
        }
        TextField {
            id: search
            objectName: "timezone-search"
            Layout.fillWidth: true
            Layout.preferredHeight: Math.round(43 * root.uiScale)
            font.pixelSize: Math.round(15 * root.uiScale)
            color: "#111111"
            selectionColor: "#820101"
            selectedTextColor: "white"
            // Leave 5-6px inside the 1-2px border, even at compact sizes.
            leftPadding: 7
            rightPadding: leftPadding
            placeholderText: root.searchPlaceholder
            placeholderTextColor: "transparent"
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
            LayoutMirroring.enabled: false
            selectByMouse: true
            Text {
                objectName: "timezone-search-placeholder"
                anchors.fill: parent
                anchors.leftMargin: search.leftPadding
                anchors.rightMargin: search.rightPadding
                visible: !search.text.length && !search.preeditText.length
                text: search.placeholderText
                font: search.font
                color: "#82858d"
                verticalAlignment: Text.AlignVCenter
                horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
                elide: Text.ElideRight
                LayoutMirroring.enabled: false
            }
            onAccepted: root.confirm()
            onActiveFocusChanged: if (activeFocus) root.typingFocused()
            Keys.onDownPressed: event => { if (results.count) { results.forceActiveFocus(); results.choose(0) }; event.accepted = true }
            background: Rectangle {
                color: "white"
                radius: Math.round(6 * root.uiScale)
                border.color: search.activeFocus ? "#111111" : "#b9bcc2"
                border.width: search.activeFocus ? 2 : 1
            }
        }
        LanguagePicker {
            id: results
            objectName: "timezone-results"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: root.minimumResultsHeight
            languages: root.filteredZones
            selectedCode: root.selectedCode
            allowUnselectedContinue: root.canConfirm
            labelLanguage: root.language
            rtl: root.rtl
            uiScale: root.uiScale
            onLanguageSelected: id => root.zoneSelected(id)
            onContinueRequested: root.confirm()
            Text {
                anchors.fill: parent
                anchors.rightMargin: results.rightMargin
                anchors.leftMargin: results.leftMargin
                visible: results.count === 0
                text: root.emptyText
                wrapMode: Text.Wrap
                font.pixelSize: Math.round(15 * root.uiScale)
                color: "#55565a"
                horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
                LayoutMirroring.enabled: false
            }
        }
    }
}
