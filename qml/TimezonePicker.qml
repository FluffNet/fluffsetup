import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtPositioning

Item {
    id: root
    property var zones: []
    property string selectedCode: "UTC"
    property string language: "en"
    property string region: ""
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
    readonly property real resultsGutter: results.verticalScrollBar.width + Math.round(8 * uiScale)
    readonly property var mapItem: mapLoader.item
    readonly property var initialMapZones: zones.filter(zone => zone.suggested).map(zone => zone.code)
    // Give large regions a useful initial view instead of averaging distant
    // territories. Any selected timezone takes precedence over this framing.
    readonly property var initialMapBounds: {
        if (selectedCode.length) return null
        if (region === "US")
            return QtPositioning.rectangle(QtPositioning.coordinate(50, -125), QtPositioning.coordinate(24, -66))
        // Mainland, Torres Strait islands, Tasmania and Lord Howe. Macquarie
        // remains in the list and fits independently when chosen.
        if (region === "AU")
            return QtPositioning.rectangle(QtPositioning.coordinate(-9, 112), QtPositioning.coordinate(-44.5, 160))
        // Include the main and southern islands plus Chatham across the date
        // line, rather than centering an arithmetic longitude average at 0.
        if (region === "NZ")
            return QtPositioning.rectangle(QtPositioning.coordinate(-33, 165), QtPositioning.coordinate(-53.5, -175))
        if (region === "ID")
            return QtPositioning.rectangle(QtPositioning.coordinate(7, 94), QtPositioning.coordinate(-12, 142))
        // Mainland and Easter Island; the selected Punta Arenas area can
        // still fit its detached Antarctic territory independently.
        if (region === "CL")
            return QtPositioning.rectangle(QtPositioning.coordinate(-17, -111), QtPositioning.coordinate(-57, -66))
        return null
    }
    readonly property var selectedCenter: {
        const selected = zones.find(zone => zone.code === selectedCode)
        if (selected && selected.center) return selected.center
        if (selectedCode.length) return null
        const suggested = zones.filter(zone => zone.suggested && zone.center)
        if (!suggested.length) return null
        // Longitudes wrap at the date line. A circular mean keeps nearby
        // islands at +179/-179 together instead of placing them near 0.
        const radians = Math.PI / 180
        const east = suggested.reduce((sum, zone) => sum + Math.sin(zone.center[1] * radians), 0)
        const north = suggested.reduce((sum, zone) => sum + Math.cos(zone.center[1] * radians), 0)
        const longitude = Math.hypot(east, north) > 0.000001
            ? Math.atan2(east, north) / radians : suggested[0].center[1]
        return [suggested.reduce((sum, zone) => sum + zone.center[0], 0) / suggested.length, longitude]
    }
    readonly property real minimumResultsHeight: 5 * Math.round(44 * uiScale) + 4 * Math.round(4 * uiScale)
    readonly property string selectedLabel: {
        const row = zones.find(zone => zone.code === selectedCode)
        return row ? row.label : selectedCode.length ? selectedCode.replace(/_/g, " ") : root.mapText("Placeholder for empty time zone combobox selector", "Choose…")
    }
    readonly property var filteredZones: {
        const tokens = searchKey(search.text).trim().split(/\s+/).filter(token => token.length)
        // Rust supplies regional suggestions first, then every other timezone
        // alphabetically by its localized label. Keep that complete ordering.
        if (!tokens.length) return zones
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
                        initialBounds: Qt.binding(() => root.initialMapBounds),
                        initialZones: Qt.binding(() => root.initialMapZones),
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
        Item {
            id: resultsFrame
            objectName: "timezone-results-frame"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: root.minimumResultsHeight
            LayoutMirroring.enabled: false
            LanguagePicker {
                id: results
                objectName: "timezone-results"
                anchors.fill: parent
                anchors.leftMargin: root.rtl ? root.resultsGutter : 0
                anchors.rightMargin: root.rtl ? 0 : root.resultsGutter
                // Keep the scrollbar outside the clipped row viewport. Native
                // left/right Flickable margins are not needed in this frame.
                leftMargin: 0
                rightMargin: 0
                languages: root.filteredZones
                selectedCode: root.selectedCode
                allowUnselectedContinue: root.canConfirm
                labelLanguage: root.language
                rtl: root.rtl
                uiScale: root.uiScale
                verticalScrollBar.parent: resultsFrame
                verticalScrollBar.policy: ScrollBar.AsNeeded
                verticalScrollBar.visible: count > 0 && contentHeight > height + 1
                verticalScrollBar.padding: 2
                verticalScrollBar.minimumSize: Math.min(1, Math.round(24 * root.uiScale) / Math.max(1, height))
                verticalScrollBar.contentItem: Rectangle {
                    implicitWidth: 6
                    radius: width / 2
                    color: results.verticalScrollBar.pressed ? "#6b6e74"
                           : results.verticalScrollBar.hovered ? "#82858d" : "#a7a9ad"
                }
                verticalScrollBar.background: Item {}
                onLanguageSelected: id => root.zoneSelected(id)
                onContinueRequested: root.confirm()
                Text {
                    anchors.fill: parent
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
}
