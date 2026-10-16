import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Button {
    id: layoutIndicator
    property var layouts: []
    property int activeIndex: -1
    property bool rtl: false
    property real uiScale: 1
    property real popupAvailableWidth: 960
    property real popupAvailableHeight: 600
    property real popupHorizontalOffset: rtl ? width - layoutMenu.width : 0
    property string label: "Keyboard layout"
    readonly property var activeLayout: activeIndex >= 0 && activeIndex < layouts.length ? layouts[activeIndex] : null
    readonly property bool hasMultipleLayouts: layouts.length > 1
    property alias menu: layoutMenu
    signal layoutRequested(string id)
    signal restoreTypingFocus()
    objectName: "keyboard-indicator"
    visible: hasMultipleLayouts
    implicitWidth: Math.round(252 * uiScale)
    implicitHeight: Math.round(64 * uiScale)
    readonly property string buttonLabel: (activeLayout ? activeLayout.name : label).replace(/\s+(\([^()]+\))$/, "\n$1")
    leftPadding: Math.round(12 * uiScale)
    rightPadding: leftPadding
    topPadding: Math.round(8 * uiScale)
    bottomPadding: topPadding
    focusPolicy: Qt.TabFocus
    hoverEnabled: true
    Accessible.name: label + (activeLayout ? ": " + activeLayout.name : "")
    LayoutMirroring.enabled: rtl
    LayoutMirroring.childrenInherit: true
    onClicked: {
        if (!hasMultipleLayouts) return
        if (layoutMenu.opened) layoutMenu.close()
        else {
            layoutMenu.open()
            layoutMenu.currentIndex = Math.max(0, activeIndex)
        }
    }
    onVisibleChanged: if (!visible) layoutMenu.close()
    onEnabledChanged: if (!enabled) layoutMenu.close()
    onRtlChanged: layoutMenu.close()
    onLayoutsChanged: layoutMenu.close()

    background: Rectangle {
        objectName: "keyboard-indicator-button"
        radius: Math.round(6 * layoutIndicator.uiScale)
        color: layoutIndicator.down ? "#dde0e5" : layoutIndicator.hovered ? "#e6e8ec" : "white"
        border.color: layoutIndicator.visualFocus ? "#111111" : "#b9bcc2"
        border.width: layoutIndicator.visualFocus ? 2 : 1
        opacity: layoutIndicator.enabled ? 1 : 0.5
    }
    contentItem: RowLayout {
        spacing: Math.round(8 * layoutIndicator.uiScale)
        Image {
            objectName: "keyboard-indicator-icon"
            Layout.preferredWidth: Math.round(22 * layoutIndicator.uiScale)
            Layout.preferredHeight: Math.round(22 * layoutIndicator.uiScale)
            source: Qt.resolvedUrl("../assets/keyboard-layout.svg")
            fillMode: Image.PreserveAspectFit
            opacity: layoutIndicator.enabled ? 1 : 0.5
        }
        Text {
            id: layoutName
            objectName: "keyboard-indicator-name"
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "\u2068" + layoutIndicator.buttonLabel + "\u2069"
            color: layoutIndicator.enabled ? "#222222" : "#85888e"
            font.pixelSize: Math.round(14 * layoutIndicator.uiScale)
            font.weight: Font.Medium
            wrapMode: Text.Wrap
            maximumLineCount: 2
            fontSizeMode: Text.Fit
            minimumPixelSize: Math.max(1, Math.round(6 * layoutIndicator.uiScale))
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        Text {
            id: menuArrow
            text: "▾"
            color: layoutIndicator.enabled ? "#55565a" : "#85888e"
            font.pixelSize: Math.round(14 * layoutIndicator.uiScale)
        }
    }
    ToolTip.visible: hovered
    ToolTip.text: Accessible.name
    ToolTip.delay: 500

    Popup {
        id: layoutMenu
        objectName: "keyboard-indicator-menu"
        property alias currentIndex: menuView.currentIndex
        readonly property int count: menuView.count
        function itemAt(index) { return menuView.itemAtIndex(index) }
        function chooseCurrent() {
            if (currentIndex >= 0 && currentIndex < layoutIndicator.layouts.length) {
                layoutIndicator.layoutRequested(layoutIndicator.layouts[currentIndex].id)
                close()
            }
        }
        popupType: Popup.Item
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        padding: Math.round(6 * layoutIndicator.uiScale)
        width: Math.min(Math.round(360 * layoutIndicator.uiScale), layoutIndicator.popupAvailableWidth)
        x: layoutIndicator.popupHorizontalOffset
        y: -height - Math.round(6 * layoutIndicator.uiScale)
        margins: 0
        height: Math.min(implicitHeight, layoutIndicator.popupAvailableHeight)
        background: Rectangle {
            color: "white"
            border.color: "#b9bcc2"
            radius: Math.round(8 * layoutIndicator.uiScale)
        }
        contentItem: ListView {
            id: menuView
            implicitHeight: contentHeight
            model: layoutIndicator.layouts
            interactive: contentHeight > height
            keyNavigationEnabled: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            LayoutMirroring.enabled: layoutIndicator.rtl
            LayoutMirroring.childrenInherit: true
            Keys.onReturnPressed: layoutMenu.chooseCurrent()
            Keys.onEnterPressed: layoutMenu.chooseCurrent()
            Kirigami.WheelHandler {
                target: menuView
                blockTargetWheel: true
                scrollFlickableTarget: true
                filterMouseEvents: false
            }
            ScrollBar.vertical: ScrollBar {}
            delegate: ItemDelegate {
                id: choice
                required property int index
                required property var modelData
                objectName: "keyboard-indicator-choice-" + modelData.id
                width: menuView.width
                text: modelData.name
                checked: index === layoutIndicator.activeIndex
                highlighted: index === menuView.currentIndex
                focusPolicy: Qt.NoFocus
                implicitHeight: Math.max(Math.round(44 * layoutIndicator.uiScale), contentItem.implicitHeight + topPadding + bottomPadding)
                Accessible.role: Accessible.ListItem
                Accessible.checkable: true
                Accessible.checked: checked
                background: Rectangle {
                    radius: Math.round(4 * layoutIndicator.uiScale)
                    color: choice.down ? "#dde0e5" : choice.highlighted || choice.hovered ? "#f4e5e5" : "white"
                }
                contentItem: RowLayout {
                    spacing: Math.round(8 * layoutIndicator.uiScale)
                    Text {
                        Layout.preferredWidth: Math.round(18 * layoutIndicator.uiScale)
                        text: choice.checked ? "✓" : ""
                        color: "#820101"
                        font.pixelSize: Math.round(15 * layoutIndicator.uiScale)
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "\u2068" + choice.modelData.name + "\u2069"
                        color: "#222222"
                        font.pixelSize: Math.round(14 * layoutIndicator.uiScale)
                        font.weight: choice.checked ? Font.DemiBold : Font.Normal
                        wrapMode: Text.Wrap
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: layoutIndicator.rtl ? Text.AlignRight : Text.AlignLeft
                    }
                }
                onClicked: {
                    layoutIndicator.layoutRequested(modelData.id)
                    layoutMenu.close()
                }
            }
        }
        onOpened: {
            menuView.currentIndex = Math.max(0, layoutIndicator.activeIndex)
            menuView.forceActiveFocus()
        }
        onClosed: layoutIndicator.restoreTypingFocus()
    }
}
