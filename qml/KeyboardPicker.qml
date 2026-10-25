import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: picker
    property var layouts: []
    property var selectedIds: ["us"]
    property var labels: ({selected: "Selected layouts", search: "Search layouts", default: "Default", remove: "Remove", add: "Add", empty: "No matching layouts", suggested: "Suggested layouts", other: "Other layouts", moveUp: "Move up", moveDown: "Move down", limit: "You can select up to four layouts."})
    property bool rtl: false
    property real uiScale: 1
    property alias searchText: searchField.text
    property alias resultsView: results
    property alias verticalScrollBar: scrollbar
    property alias searchInput: searchField
    readonly property var selectedLayouts: selectedIds.map(id => layouts.find(layout => layout.id === id)).filter(layout => layout !== undefined)
    readonly property var filteredLayouts: layouts.filter(layout => !selectedIds.includes(layout.id) && searchKey(layout.name + " " + (layout.search || "") + " " + layout.id).includes(searchKey(searchField.text)))
    readonly property bool hasSuggestions: filteredLayouts.some(layout => layout.group === "suggested")
    property string draggedId: ""
    property int dragDestination: -1
    property real dragVisualY: 0
    property real dragOffsetY: 0
    property int dragSerial: 0
    signal addRequested(string id)
    signal removeRequested(string id)
    signal moveRequested(string id, int direction)
    signal reorderRequested(string id, int destination)
    signal continueRequested()
    spacing: Math.round(12 * uiScale)
    LayoutMirroring.enabled: rtl
    LayoutMirroring.childrenInherit: true

    function searchKey(value) {
        return value.trim().toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "")
    }
    function focusSearch() { searchField.forceActiveFocus() }
    function cancelSelectedDrag() {
        ++dragSerial
        draggedId = ""
        dragDestination = -1
    }
    function beginSelectedDrag(id, scenePosition) {
        cancelSelectedDrag()
        const row = selectedRepeater.itemAt(selectedIds.indexOf(id))
        if (!row || selectedIds.length < 2) return
        const point = selectedColumn.mapFromItem(null, scenePosition.x, scenePosition.y)
        draggedId = id
        dragOffsetY = point.y - row.y
        updateSelectedDrag(scenePosition)
        picker.forceActiveFocus(Qt.MouseFocusReason)
    }
    function updateSelectedDrag(scenePosition) {
        if (!draggedId) return
        const point = selectedColumn.mapFromItem(null, scenePosition.x, scenePosition.y)
        const row = selectedRepeater.itemAt(selectedIds.indexOf(draggedId))
        if (!row) { cancelSelectedDrag(); return }
        dragVisualY = Math.max(0, Math.min(selectedColumn.height - row.height, point.y - dragOffsetY))
        const margin = 32 * uiScale
        if (point.x < -margin || point.x > selectedColumn.width + margin
                || point.y < -margin || point.y > selectedColumn.height + margin) {
            dragDestination = -1
            return
        }
        let nearest = -1
        let distance = Infinity
        for (let index = 0; index < selectedRepeater.count; ++index) {
            const candidate = selectedRepeater.itemAt(index)
            if (!candidate) continue
            const delta = Math.abs(point.y - candidate.y - candidate.height / 2)
            if (delta < distance) { distance = delta; nearest = index }
        }
        dragDestination = nearest
    }
    function finishSelectedDrag(serial) {
        if (serial !== dragSerial || !draggedId) return
        const id = draggedId
        const destination = dragDestination
        const source = selectedIds.indexOf(id)
        cancelSelectedDrag()
        if (!visible || !enabled || source < 0 || destination < 0
                || destination >= selectedIds.length || destination === source) return
        // Commit once after the pointer releases, so the repeater is never
        // rebuilt under an active grab and KWin sees one final ordered update.
        reorderRequested(id, destination)
        Qt.callLater(function() {
            const row = selectedRepeater.itemAt(selectedIds.indexOf(id))
            if (!row) return
            const button = row.upButton.enabled ? row.upButton : row.downButton
            if (button.enabled) button.forceActiveFocus(Qt.MouseFocusReason)
            else picker.focusSearch()
        })
    }
    onSelectedIdsChanged: cancelSelectedDrag()
    onSelectedLayoutsChanged: cancelSelectedDrag()
    onRtlChanged: cancelSelectedDrag()
    onUiScaleChanged: cancelSelectedDrag()
    onWidthChanged: cancelSelectedDrag()
    onHeightChanged: cancelSelectedDrag()
    onVisibleChanged: if (!visible) cancelSelectedDrag()
    onEnabledChanged: if (!enabled) cancelSelectedDrag()
    Keys.onEscapePressed: event => {
        if (draggedId) { cancelSelectedDrag(); event.accepted = true }
        else event.accepted = false
    }
    function moveSelected(id, direction, reason) {
        moveRequested(id, direction)
        // Reordering rebuilds the selected-row repeater. Keep keyboard focus
        // on the moved layout, including when it reaches the top or bottom.
        Qt.callLater(function() {
            const index = selectedIds.indexOf(id)
            const row = selectedRepeater.itemAt(index)
            if (!row) return
            let button = direction < 0 ? row.upButton : row.downButton
            if (!button.enabled) button = direction < 0 ? row.downButton : row.upButton
            if (!button.enabled) { picker.focusSearch(); return }
            button.forceActiveFocus(reason)
        })
    }
    function addCurrent() {
        if (results.currentIndex >= 0 && results.currentIndex < filteredLayouts.length && selectedIds.length < 4)
            addRequested(filteredLayouts[results.currentIndex].id)
    }
    function navigate(index) {
        results.currentIndex = filteredLayouts.length ? Math.min(filteredLayouts.length - 1, Math.max(0, index)) : -1
    }
    onFilteredLayoutsChanged: {
        Qt.callLater(function() {
            navigate(results.currentIndex)
            if (results.currentIndex >= 0) results.positionViewAtIndex(results.currentIndex, ListView.Contain)
        })
    }

    Text {
        Layout.fillWidth: true
        text: picker.labels.selected
        font.pixelSize: Math.round(15 * picker.uiScale)
        color: "#55565a"
        horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
        LayoutMirroring.enabled: false
        wrapMode: Text.Wrap
    }
    // At most four rows. They remain fully visible, with no nested scrolling.
    Item {
        id: selectedContainer
        Layout.fillWidth: true
        implicitHeight: selectedColumn.implicitHeight
        ColumnLayout {
            id: selectedColumn
            width: parent.width
            spacing: Math.round(4 * picker.uiScale)
            Repeater {
                id: selectedRepeater
                model: picker.selectedLayouts
                delegate: Rectangle {
                    id: selectedRow
                    required property var modelData
                    required property int index
                    property alias upButton: moveUpButton
                    property alias downButton: moveDownButton
                    readonly property bool isDefault: index === 0
                    objectName: "selected-row-" + modelData.id
                    Layout.fillWidth: true
                    implicitHeight: Math.max(48 * picker.uiScale, selectedName.implicitHeight + 18 * picker.uiScale)
                    radius: Math.round(6 * picker.uiScale)
                    color: picker.draggedId === modelData.id ? "#fff3f3" : "white"
                    border.color: picker.draggedId === modelData.id ? "#820101" : "#b9bcc2"
                    z: picker.draggedId === modelData.id ? 10 : 0
                    transform: Translate { y: picker.draggedId === selectedRow.modelData.id ? picker.dragVisualY - selectedRow.y : 0 }
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: Math.round(6 * picker.uiScale)
                        anchors.leftMargin: Math.round(12 * picker.uiScale)
                        anchors.rightMargin: Math.round(12 * picker.uiScale)
                        spacing: Math.round(8 * picker.uiScale)
                        Item {
                            id: dragArea
                            objectName: "drag-" + selectedRow.modelData.id
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            implicitHeight: selectedName.implicitHeight
                            Accessible.role: Accessible.ListItem
                            Accessible.name: selectedName.text
                            HoverHandler { cursorShape: picker.selectedIds.length > 1 ? Qt.OpenHandCursor : Qt.ArrowCursor }
                            Item {
                                width: Math.round(14 * picker.uiScale)
                                height: Math.round(18 * picker.uiScale)
                                x: picker.rtl ? parent.width - width : 0
                                anchors.verticalCenter: parent.verticalCenter
                                visible: picker.selectedIds.length > 1
                                Repeater {
                                    model: 6
                                    Rectangle {
                                        required property int index
                                        width: Math.max(2, Math.round(3 * picker.uiScale))
                                        height: width
                                        radius: width / 2
                                        x: (index % 2) * 6 * picker.uiScale
                                        y: Math.floor(index / 2) * 6 * picker.uiScale
                                        color: "#81848a"
                                    }
                                }
                            }
                            Text {
                                id: selectedName
                                objectName: "selected-name-" + selectedRow.modelData.id
                                anchors.verticalCenter: parent.verticalCenter
                                x: picker.rtl || picker.selectedIds.length === 1 ? 0 : 20 * picker.uiScale
                                width: parent.width - (picker.selectedIds.length > 1 ? 20 * picker.uiScale : 0)
                                text: "\u2068" + selectedRow.modelData.name + "\u2069" + (selectedRow.index === 0 ? " - " + picker.labels.default : "")
                                font.pixelSize: Math.round(15 * picker.uiScale)
                                font.weight: selectedRow.index === 0 ? Font.DemiBold : Font.Normal
                                color: "#111111"
                                wrapMode: Text.Wrap
                                horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
                                LayoutMirroring.enabled: false
                            }
                            DragHandler {
                                id: selectedDrag
                                property int transactionSerial: -1
                                readonly property point scenePointer: centroid.scenePosition
                                target: null
                                acceptedButtons: Qt.LeftButton
                                minimumPointCount: 1
                                maximumPointCount: 1
                                xAxis.enabled: false
                                enabled: picker.selectedIds.length > 1 && (!picker.draggedId || picker.draggedId === selectedRow.modelData.id)
                                cursorShape: Qt.ClosedHandCursor
                                onActiveChanged: {
                                    if (active) {
                                        picker.beginSelectedDrag(selectedRow.modelData.id, centroid.scenePosition)
                                        transactionSerial = picker.dragSerial
                                    }
                                    else {
                                        const serial = transactionSerial
                                        Qt.callLater(function() { picker.finishSelectedDrag(serial) })
                                    }
                                }
                                onScenePointerChanged: if (active && transactionSerial === picker.dragSerial) picker.updateSelectedDrag(scenePointer)
                                onCanceled: if (transactionSerial === picker.dragSerial) picker.cancelSelectedDrag()
                            }
                        }
                        MoveButton {
                            id: moveUpButton
                            layoutId: selectedRow.modelData.id
                            layoutName: selectedRow.modelData.name
                            upward: true
                            enabled: selectedRow.index > 0
                        }
                        MoveButton {
                            id: moveDownButton
                            layoutId: selectedRow.modelData.id
                            layoutName: selectedRow.modelData.name
                            upward: false
                            enabled: selectedRow.index < picker.selectedIds.length - 1
                        }
                        ToolButton {
                            id: removeButton
                            objectName: "remove-" + selectedRow.modelData.id
                            implicitWidth: Math.round(36 * picker.uiScale)
                            implicitHeight: Math.round(32 * picker.uiScale)
                            padding: 0
                            contentItem: Item {
                                Image {
                                    objectName: "remove-icon-" + selectedRow.modelData.id
                                    anchors.centerIn: parent
                                    width: Math.round(22 * picker.uiScale)
                                    height: width
                                    sourceSize: Qt.size(width, height)
                                    source: Qt.resolvedUrl("../assets/edit-delete.svg")
                                    fillMode: Image.PreserveAspectFit
                                    opacity: removeButton.enabled ? 1 : 0.4
                                }
                            }
                            enabled: picker.selectedIds.length > 1
                            Accessible.name: picker.labels.remove + ": " + selectedRow.modelData.name
                            ToolTip.text: picker.labels.remove
                            ToolTip.visible: hovered
                            onClicked: picker.removeRequested(selectedRow.modelData.id)
                        }
                    }
                }
            }
        }
        Rectangle {
            objectName: "keyboard-drop-indicator"
            visible: picker.draggedId.length > 0 && picker.dragDestination >= 0
                     && picker.dragDestination !== picker.selectedIds.indexOf(picker.draggedId)
            width: parent.width
            height: Math.max(2, Math.round(3 * picker.uiScale))
            color: "#820101"
            z: 20
            y: {
                const row = selectedRepeater.itemAt(picker.dragDestination)
                if (!row) return 0
                return row.y + (picker.dragDestination > picker.selectedIds.indexOf(picker.draggedId) ? row.height : 0) - height / 2
            }
        }
    }
    component MoveButton: ToolButton {
        id: control
        required property string layoutId
        required property string layoutName
        required property bool upward
        objectName: (upward ? "move-up-" : "move-down-") + layoutId
        implicitWidth: Math.round(32 * picker.uiScale)
        implicitHeight: Math.round(32 * picker.uiScale)
        padding: 0
        activeFocusOnTab: true
        Keys.onReturnPressed: event => { if (enabled) clicked(); event.accepted = true }
        Keys.onEnterPressed: event => { if (enabled) clicked(); event.accepted = true }
        Accessible.name: (upward ? picker.labels.moveUp : picker.labels.moveDown) + ": " + layoutName
        ToolTip.text: upward ? picker.labels.moveUp : picker.labels.moveDown
        ToolTip.visible: hovered
        onClicked: picker.moveSelected(layoutId, upward ? -1 : 1, focusReason)
        contentItem: Item {
            Repeater {
                model: 2
                Rectangle {
                    required property int index
                    width: Math.round(9 * picker.uiScale)
                    height: Math.max(1, Math.round(2 * picker.uiScale))
                    x: (parent.width - width) / 2 + (index === 0 ? -3 : 3) * picker.uiScale
                    y: (parent.height - height) / 2
                    rotation: (index === 0 ? -45 : 45) * (control.upward ? 1 : -1)
                    color: control.enabled ? "#111111" : "#b9bcc2"
                }
            }
        }
        background: Rectangle {
            radius: Math.round(4 * picker.uiScale)
            color: control.down ? "#dde0e5" : control.hovered ? "#eef0f3" : "transparent"
            border.color: "#111111"
            border.width: control.visualFocus ? 2 : 0
        }
    }
    TextField {
        id: searchField
        objectName: "keyboard-search"
        Layout.fillWidth: true
        implicitHeight: Math.max(Math.round(44 * picker.uiScale), searchPlaceholder.implicitHeight + Math.round(16 * picker.uiScale))
        font.pixelSize: Math.round(15 * picker.uiScale)
        color: "#111111"
        horizontalAlignment: picker.rtl ? TextInput.AlignRight : TextInput.AlignLeft
        LayoutMirroring.enabled: false
        leftPadding: Math.round(12 * picker.uiScale)
        rightPadding: leftPadding
        selectByMouse: true
        hoverEnabled: true
        placeholderText: picker.labels.search
        placeholderTextColor: "transparent"
        Text {
            id: searchPlaceholder
            objectName: "keyboard-search-placeholder"
            anchors.verticalCenter: parent.verticalCenter
            x: searchField.leftPadding
            width: searchField.width - searchField.leftPadding - searchField.rightPadding
            visible: searchField.length === 0 && searchField.preeditText.length === 0
            text: searchField.placeholderText
            font: searchField.font
            color: "#81848a"
            horizontalAlignment: searchField.horizontalAlignment
            verticalAlignment: Text.AlignVCenter
            LayoutMirroring.enabled: false
            wrapMode: Text.Wrap
        }
        background: Rectangle {
            radius: Math.round(7 * picker.uiScale)
            color: "white"
            border.color: searchField.activeFocus ? "#111111" : "#b9bcc2"
            border.width: searchField.activeFocus ? 3 : 1
        }
        KeyNavigation.tab: results
        Keys.onDownPressed: {
            if (results.count) results.forceActiveFocus()
        }
        Keys.onReturnPressed: {
            if (text.trim().length === 0) picker.continueRequested()
            else picker.addCurrent()
        }
        Keys.onEnterPressed: {
            if (text.trim().length === 0) picker.continueRequested()
            else picker.addCurrent()
        }
    }
    Text {
        objectName: "keyboard-section-other"
        Layout.fillWidth: true
        visible: picker.filteredLayouts.length > 0 && !picker.hasSuggestions
        text: picker.labels.other
        font.pixelSize: Math.round(15 * picker.uiScale)
        color: "#55565a"
        wrapMode: Text.Wrap
        horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
        LayoutMirroring.enabled: false
    }
    ListView {
        id: results
        objectName: "keyboard-results"
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: Math.round(60 * picker.uiScale)
        model: picker.filteredLayouts
        section.property: picker.hasSuggestions ? "group" : ""
        section.criteria: ViewSection.FullString
        section.delegate: Text {
            required property string section
            objectName: "keyboard-section-" + section
            width: results.width - results.leftMargin - results.rightMargin
            height: implicitHeight + Math.round(12 * picker.uiScale)
            text: section === "suggested" ? picker.labels.suggested : picker.labels.other
            font.pixelSize: Math.round(15 * picker.uiScale)
            color: "#55565a"
            wrapMode: Text.Wrap
            horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
            verticalAlignment: Text.AlignVCenter
            LayoutMirroring.enabled: false
        }
        clip: true
        // Keep native touch gestures; the wheel handler must not replace them.
        interactive: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        activeFocusOnTab: true
        spacing: Math.round(4 * picker.uiScale)
        leftMargin: picker.rtl ? scrollbar.width + 8 * picker.uiScale : 0
        rightMargin: picker.rtl ? 0 : scrollbar.width + 8 * picker.uiScale
        LayoutMirroring.enabled: false
        keyNavigationEnabled: false
        currentIndex: count ? 0 : -1
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Up:
                if (currentIndex <= 0) picker.focusSearch()
                else picker.navigate(currentIndex - 1)
                break
            case Qt.Key_Down: picker.navigate(currentIndex + 1); break
            case Qt.Key_Home: picker.navigate(0); break
            case Qt.Key_End: picker.navigate(count - 1); break
            case Qt.Key_PageUp: picker.navigate(currentIndex - 6); break
            case Qt.Key_PageDown: picker.navigate(currentIndex + 6); break
            case Qt.Key_Return:
            case Qt.Key_Enter: picker.addCurrent(); break
            default: return
            }
            if (currentIndex >= 0) positionViewAtIndex(currentIndex, ListView.Contain)
            event.accepted = true
        }
        delegate: ItemDelegate {
            id: availableRow
            required property var modelData
            required property int index
            objectName: "add-" + modelData.id
            width: results.width - results.leftMargin - results.rightMargin
            height: Math.max(44 * picker.uiScale, availableName.implicitHeight + 20 * picker.uiScale)
            enabled: picker.selectedIds.length < 4
            padding: 10 * picker.uiScale
            focusPolicy: Qt.NoFocus
            Accessible.name: picker.labels.add + ": " + modelData.name
            onClicked: {
                results.currentIndex = index
                picker.addRequested(modelData.id)
            }
            contentItem: RowLayout {
                LayoutMirroring.enabled: picker.rtl
                Text {
                    id: availableName
                    Layout.fillWidth: true
                    text: "\u2068" + availableRow.modelData.name + "\u2069"
                    font.pixelSize: Math.round(15 * picker.uiScale)
                    color: availableRow.enabled ? "#111111" : "#81848a"
                    wrapMode: Text.Wrap
                    horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
                    LayoutMirroring.enabled: false
                }
                Text {
                    objectName: "add-icon-" + availableRow.modelData.id
                    text: "+"
                    color: availableRow.enabled ? "#16823e" : "#81848a"
                    font.pixelSize: Math.round(22 * picker.uiScale)
                }
            }
            background: Rectangle {
                radius: Math.round(6 * picker.uiScale)
                color: "white"
                border.color: results.activeFocus && results.currentIndex === availableRow.index ? "#111111" : "#d0d2d7"
                border.width: results.activeFocus && results.currentIndex === availableRow.index ? 3 : 1
            }
        }
        Text {
            anchors.fill: parent
            visible: results.count === 0
            text: picker.labels.empty
            font.pixelSize: Math.round(15 * picker.uiScale)
            color: "#55565a"
            wrapMode: Text.Wrap
            horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
            LayoutMirroring.enabled: false
        }
        ScrollBar.vertical: ScrollBar {
            id: scrollbar
            objectName: "keyboardResultsScrollBar"
            parent: results
            x: picker.rtl ? 0 : results.width - width
            y: 0
            height: results.height
            width: Math.max(8, Math.round(8 * picker.uiScale))
            policy: ScrollBar.AlwaysOn
            visible: results.count > 0 && results.contentHeight > results.height + 1
            minimumSize: 0.04
        }
        Kirigami.WheelHandler {
            target: results
            blockTargetWheel: true
            scrollFlickableTarget: true
            filterMouseEvents: false
        }
    }
    Text {
        Layout.fillWidth: true
        visible: picker.selectedIds.length >= 4
        text: picker.labels.limit
        font.pixelSize: Math.round(13 * picker.uiScale)
        color: "#55565a"
        wrapMode: Text.Wrap
        horizontalAlignment: picker.rtl ? Text.AlignRight : Text.AlignLeft
        LayoutMirroring.enabled: false
    }
}
