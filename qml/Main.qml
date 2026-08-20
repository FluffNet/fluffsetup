import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.flufflinux.setup

ApplicationWindow {
    id: window
    readonly property bool compactScreen: Screen.width <= 1280 || Screen.height <= 720
    readonly property int targetWidth: compactScreen
                                       ? Screen.width
                                       : Math.round(Screen.width * 0.90)
    readonly property int targetHeight: compactScreen
                                        ? Screen.height
                                        : Math.round(Screen.height * 0.88)
    readonly property real uiScale: Math.max(0.90,
                                             Math.min(1.25,
                                                      Math.min(width / 960,
                                                               height / 640)))
    readonly property int pageHorizontalMargin: Math.round(62 * uiScale)
    readonly property int pageTopMargin: Math.round(68 * uiScale)
    readonly property int pageBottomMargin: Math.round(48 * uiScale)

    width: targetWidth
    height: targetHeight
    minimumWidth: targetWidth
    maximumWidth: targetWidth
    minimumHeight: targetHeight
    maximumHeight: targetHeight
    visible: true
    title: "Fluff Linux Setup"
    color: "transparent"
    modality: Qt.ApplicationModal
    flags: Qt.Dialog | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    x: Screen.virtualX + Math.round((Screen.width - width) / 2)
    y: Screen.virtualY + Math.round((Screen.height - height) / 2)

    readonly property color accent: "#820101"
    readonly property color headingColor: "#111111"
    readonly property color bodyColor: "#55565a"
    readonly property color borderColor: "#b9bcc2"

    // Empty fields resolve to the values shown by their placeholders, so the
    // recovery record and final summary never receive an accidental blank.
    function effectiveHostname() {
        return hostnameField.text.length > 0
                ? hostnameField.text
                : hostnameField.placeholderText
    }

    function effectiveName() {
        return nameField.text.trim().length > 0
                ? nameField.text
                : nameField.placeholderText
    }

    function saveRecoveryPage(page) {
        backend.saveProgress(page, effectiveHostname(), effectiveName())
    }

    function showPage(page, focusItem) {
        saveRecoveryPage(page)
        pages.currentIndex = page
        if (focusItem)
            Qt.callLater(function() { focusItem.forceActiveFocus() })
    }

    function restoreFocus() {
        if (pages.currentIndex === 0)
            welcomePage.forceActiveFocus()
        else if (pages.currentIndex === 1)
            hostnameField.forceActiveFocus()
        else if (pages.currentIndex === 2)
            nameField.forceActiveFocus()
        else if (pages.currentIndex === 4)
            summaryPage.forceActiveFocus()
    }

    Component.onCompleted: {
        pages.currentIndex = backend.resumePage()
        Qt.callLater(restoreFocus)
    }

    // Coalesce typing into small recovery updates instead of writing the state
    // file on every keystroke.
    Timer {
        id: recoverySaveTimer
        interval: 300
        repeat: false
        onTriggered: {
            if (pages.currentIndex === 1 || pages.currentIndex === 2)
                window.saveRecoveryPage(pages.currentIndex)
        }
    }

    SetupBackend {
        id: backend
    }

    Connections {
        target: backend

        function onCompletedChanged() {
            if (backend.completed) {
                pages.currentIndex = 4
                Qt.callLater(function() { summaryPage.forceActiveFocus() })
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 0
        color: "#f7f8fa"
        clip: true
        border.color: "#d6d8dc"
        border.width: 1

        RowLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.preferredWidth: Math.round(300 * window.uiScale)
                Layout.fillHeight: true
                color: "#eef0f3"

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Math.round(36 * window.uiScale)
                    spacing: Math.round(12 * window.uiScale)

                    Item { Layout.fillHeight: true }

                    Item {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.preferredWidth: Math.round(176 * window.uiScale)
                        Layout.preferredHeight: Math.round(176 * window.uiScale)

                        Image {
                            anchors.fill: parent
                            visible: pages.currentIndex !== 1 && pages.currentIndex !== 2
                            source: "file:///usr/share/pixmaps/flufflinux-logo.svg"
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                        }

                        ToolButton {
                            anchors.centerIn: parent
                            width: parent.width
                            height: parent.height
                            visible: pages.currentIndex === 1
                            display: AbstractButton.IconOnly
                            icon.name: "network-wired"
                            icon.width: Math.round(148 * window.uiScale)
                            icon.height: Math.round(148 * window.uiScale)
                            hoverEnabled: false
                            focusPolicy: Qt.NoFocus
                            background: Item {}
                        }

                        ToolButton {
                            anchors.centerIn: parent
                            width: parent.width
                            height: parent.height
                            visible: pages.currentIndex === 2
                            display: AbstractButton.IconOnly
                            icon.name: "user-identity"
                            icon.width: Math.round(148 * window.uiScale)
                            icon.height: Math.round(148 * window.uiScale)
                            hoverEnabled: false
                            focusPolicy: Qt.NoFocus
                            background: Item {}
                        }
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        visible: pages.currentIndex !== 1 && pages.currentIndex !== 2
                        text: "Fluff Linux"
                        color: window.headingColor
                        font.pixelSize: Math.round(24 * window.uiScale)
                        font.weight: Font.DemiBold
                    }

                    Item { Layout.fillHeight: true }
                }

                Text {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: Math.round(18 * window.uiScale)
                    anchors.bottomMargin: Math.round(12 * window.uiScale)
                    visible: pages.currentIndex === 0
                    text: "fluffsetup 1.0"
                    color: "#67686c"
                    font.pixelSize: Math.round(12 * window.uiScale)
                }
            }

            StackLayout {
                id: pages
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: 0

                // Welcome
                Item {
                    id: welcomePage
                    focus: pages.currentIndex === 0
                    KeyNavigation.tab: welcomeNextButton
                    KeyNavigation.backtab: welcomeNextButton
                    Keys.onReturnPressed: event => {
                        welcomeNextButton.clicked()
                        event.accepted = true
                    }
                    Keys.onEnterPressed: event => {
                        welcomeNextButton.clicked()
                        event.accepted = true
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(14 * window.uiScale)

                        Item { Layout.fillHeight: true }

                        PageHeading {
                            text: "Welcome to Fluff Linux!"
                            font.pixelSize: Math.round(36 * window.uiScale)
                            horizontalAlignment: Text.AlignHCenter
                        }
                        PageDescription {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.maximumWidth: Math.round(680 * window.uiScale)
                            text: "Thank you for choosing Fluff Linux. This program will guide you through setting up your system and your user easily."
                            color: window.headingColor
                            font.pixelSize: Math.round(20 * window.uiScale)
                            lineHeight: 1.35
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Item { Layout.fillHeight: true }

                        PrimaryButton {
                            id: welcomeNextButton
                            Layout.alignment: Qt.AlignRight
                            text: "Next  →"
                            onClicked: window.showPage(1, hostnameField)
                        }
                    }
                }

                // System name
                Item {
                    id: hostnamePage
                    property string validationError: backend.validateHostname(hostnameField.text)

                    function continueForward() {
                        if (validationError.length === 0)
                            window.showPage(2, nameField)
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(12 * window.uiScale)

                        Item { Layout.fillHeight: true }

                        ColumnLayout {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.fillWidth: true
                            Layout.maximumWidth: Math.round(760 * window.uiScale)
                            spacing: Math.round(12 * window.uiScale)

                            PageHeading {
                                text: "Name your system"
                                font.pixelSize: Math.round(36 * window.uiScale)
                                horizontalAlignment: Text.AlignHCenter
                            }
                            PageDescription {
                                text: "Choose the name this system will use on the network.\nIf you aren’t sure, you can press the Next button."
                                color: window.headingColor
                                font.pixelSize: Math.round(20 * window.uiScale)
                                lineHeight: 1.35
                                horizontalAlignment: Text.AlignHCenter
                            }
                            Item { Layout.preferredHeight: Math.round(8 * window.uiScale) }
                            FieldLabel { text: "System name" }

                            SetupField {
                                id: hostnameField
                                Layout.fillWidth: true
                                // Keep one extra character so the validator can explain
                                // the 255 character limit instead of silently truncating.
                                maximumLength: 256
                                text: {
                                    const saved = backend.savedHostname()
                                    const fallback = backend.currentHostname()
                                    return saved.length > 0 && saved !== fallback ? saved : ""
                                }
                                placeholderText: {
                                    const saved = backend.savedHostname()
                                    return saved.length > 0 ? saved : backend.currentHostname()
                                }
                                KeyNavigation.tab: hostnamePage.validationError.length === 0
                                                   ? hostnameNextButton
                                                   : hostnameBackButton
                                onTextEdited: recoverySaveTimer.restart()
                                onAccepted: hostnamePage.continueForward()
                            }

                            Text {
                                Layout.fillWidth: true
                                text: "Examples for system names: harry-pc, FL-IW2098, LindaLaptop"
                                color: window.headingColor
                                font.pixelSize: Math.round(15 * window.uiScale)
                                wrapMode: Text.WordWrap
                            }

                            ErrorText {
                                text: hostnamePage.validationError
                                visible: text.length > 0
                            }
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true

                            SecondaryButton {
                                id: hostnameBackButton
                                text: "←  Back"
                                KeyNavigation.tab: hostnameField
                                onClicked: window.showPage(0, welcomeNextButton)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: hostnameNextButton
                                text: "Next  →"
                                enabled: hostnamePage.validationError.length === 0
                                KeyNavigation.tab: hostnameBackButton
                                onClicked: hostnamePage.continueForward()
                            }
                        }
                    }
                }

                // Permanent user details
                Item {
                    id: accountPage
                    property bool passwordsVisible: true
                    property string nameError: backend.validateName(nameField.text)
                    property string visibleError: {
                        if (nameError.length > 0)
                            return nameError
                        if (confirmPasswordField.text.length > 0
                                && passwordField.text !== confirmPasswordField.text)
                            return "The passwords do not match"
                        return ""
                    }
                    property bool canContinue: nameError.length === 0
                                               && passwordField.text.length > 0
                                               && confirmPasswordField.text.length > 0
                                               && passwordField.text === confirmPasswordField.text

                    function continueForward() {
                        if (!canContinue || backend.busy)
                            return

                        window.saveRecoveryPage(2)
                        pages.currentIndex = 3
                        backend.startSetup(window.effectiveHostname(),
                                           window.effectiveName(),
                                           passwordField.text)
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: Math.round(56 * window.uiScale)
                        anchors.bottomMargin: Math.round(44 * window.uiScale)
                        spacing: Math.round(10 * window.uiScale)

                        PageHeading { text: "Create your user" }
                        PageDescription {
                            text: "Please enter a name and password for your user."
                        }
                        FieldLabel { text: "Name" }

                        SetupField {
                            id: nameField
                            Layout.fillWidth: true
                            maximumLength: 128
                            text: {
                                const saved = backend.savedName()
                                return saved.length > 0 && saved !== backend.defaultName()
                                       ? saved
                                       : ""
                            }
                            placeholderText: backend.defaultName()
                            enabled: !backend.accountAlreadyCreated()
                            KeyNavigation.tab: passwordVisibilityButton
                            onTextEdited: recoverySaveTimer.restart()
                            onAccepted: accountPage.continueForward()
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: Math.max(passwordNoticeText.implicitHeight,
                                                     Math.round(30 * window.uiScale))
                                            + Math.round(20 * window.uiScale)
                            radius: 7
                            color: "#820101"
                            border.color: window.accent

                            RowLayout {
                                id: passwordNoticeRow
                                anchors.fill: parent
                                anchors.margins: Math.round(10 * window.uiScale)
                                spacing: Math.round(9 * window.uiScale)

                                InformationIcon {
                                    Layout.minimumWidth: Math.round(24 * window.uiScale)
                                    Layout.preferredWidth: Math.round(24 * window.uiScale)
                                    Layout.maximumWidth: Math.round(24 * window.uiScale)
                                    Layout.minimumHeight: Math.round(24 * window.uiScale)
                                    Layout.preferredHeight: Math.round(24 * window.uiScale)
                                    Layout.maximumHeight: Math.round(24 * window.uiScale)
                                }

                                Text {
                                    id: passwordNoticeText
                                    Layout.fillWidth: true
                                    Layout.minimumWidth: 0
                                    text: "Notice: Your password is visible by default. Press the eye button to hide or show it, and make sure nobody else can see your screen."
                                    color: "white"
                                    font.pixelSize: Math.round(13 * window.uiScale)
                                    wrapMode: Text.WordWrap
                                    verticalAlignment: Text.AlignVCenter
                                }

                                ToolButton {
                                    id: passwordVisibilityButton
                                    Layout.minimumWidth: Math.round(36 * window.uiScale)
                                    Layout.preferredWidth: Math.round(36 * window.uiScale)
                                    Layout.maximumWidth: Math.round(36 * window.uiScale)
                                    Layout.minimumHeight: Math.round(30 * window.uiScale)
                                    Layout.preferredHeight: Math.round(30 * window.uiScale)
                                    Layout.maximumHeight: Math.round(30 * window.uiScale)
                                    activeFocusOnTab: true
                                    hoverEnabled: true
                                    text: accountPage.passwordsVisible
                                          ? "Hide password"
                                          : "Show password"
                                    display: AbstractButton.IconOnly
                                    KeyNavigation.tab: passwordField
                                    ToolTip.visible: hovered
                                    ToolTip.text: text
                                    onClicked: accountPage.passwordsVisible = !accountPage.passwordsVisible
                                    Keys.onReturnPressed: event => {
                                        passwordVisibilityButton.clicked()
                                        event.accepted = true
                                    }
                                    Keys.onEnterPressed: event => {
                                        passwordVisibilityButton.clicked()
                                        event.accepted = true
                                    }
                                    Keys.onSpacePressed: event => {
                                        passwordVisibilityButton.clicked()
                                        event.accepted = true
                                    }
                                    contentItem: PasswordEyeIcon {
                                        opened: accountPage.passwordsVisible
                                    }
                                    background: Rectangle {
                                        radius: 5
                                        color: passwordVisibilityButton.down
                                               ? "#5f0000"
                                               : passwordVisibilityButton.hovered
                                                 ? "#710000"
                                                 : "transparent"
                                        border.width: passwordVisibilityButton.visualFocus ? 3 : 1
                                        border.color: passwordVisibilityButton.visualFocus
                                                      ? "#111111"
                                                      : "white"
                                    }
                                }
                            }
                        }

                        FieldLabel { text: "Password" }
                        SetupField {
                            id: passwordField
                            Layout.fillWidth: true
                            echoMode: accountPage.passwordsVisible
                                      ? TextInput.Normal
                                      : TextInput.Password
                            placeholderText: accountPage.passwordsVisible
                                             ? "Visible while you type"
                                             : "Hidden while you type"
                            KeyNavigation.tab: confirmPasswordField
                            onAccepted: accountPage.continueForward()
                        }

                        FieldLabel { text: "Confirm password" }
                        SetupField {
                            id: confirmPasswordField
                            Layout.fillWidth: true
                            echoMode: accountPage.passwordsVisible
                                      ? TextInput.Normal
                                      : TextInput.Password
                            placeholderText: "Enter it again"
                            KeyNavigation.tab: accountPage.canContinue
                                               ? accountNextButton
                                               : accountBackButton
                            onAccepted: accountPage.continueForward()
                        }

                        ErrorText {
                            text: accountPage.visibleError
                            visible: text.length > 0
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true

                            SecondaryButton {
                                id: accountBackButton
                                text: "←  Back"
                                KeyNavigation.tab: nameField
                                onClicked: window.showPage(1, hostnameField)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: accountNextButton
                                text: "Next  →"
                                enabled: accountPage.canContinue && !backend.busy
                                KeyNavigation.tab: accountBackButton
                                onClicked: accountPage.continueForward()
                            }
                        }
                    }
                }

                // Applying settings
                Item {
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(14 * window.uiScale)

                        PageHeading {
                            text: backend.errorMessage.length > 0
                                  ? "Setup could not be completed"
                                  : "Setting up your system"
                        }
                        PageDescription {
                            text: backend.errorMessage.length > 0
                                  ? "Review the message below, then return to your details and try again."
                                  : "FluffSetup is applying your system name, creating your account, and preparing your desktop."
                        }

                        Item { Layout.fillHeight: true }

                        FieldLabel {
                            text: backend.errorMessage.length > 0
                                  ? "Your settings were not fully applied."
                                  : "Applying your settings…"
                        }

                        ProgressBar {
                            id: setupProgress
                            Layout.fillWidth: true
                            indeterminate: backend.busy
                            visible: backend.errorMessage.length === 0
                            background: Rectangle {
                                implicitHeight: Math.round(8 * window.uiScale)
                                radius: Math.round(4 * window.uiScale)
                                color: "#dedfe2"
                            }
                            contentItem: Item {
                                implicitHeight: Math.round(8 * window.uiScale)
                                clip: true

                                Rectangle {
                                    id: movingChunk
                                    width: Math.max(Math.round(80 * window.uiScale),
                                                    parent.width * 0.28)
                                    height: parent.height
                                    radius: Math.round(4 * window.uiScale)
                                    color: window.accent

                                    SequentialAnimation on x {
                                        running: setupProgress.visible && setupProgress.indeterminate
                                        loops: Animation.Infinite
                                        NumberAnimation {
                                            from: -movingChunk.width
                                            to: movingChunk.parent.width
                                            duration: 1150
                                            easing.type: Easing.InOutCubic
                                        }
                                    }
                                }
                            }
                        }

                        ErrorText {
                            Layout.fillWidth: true
                            text: backend.errorMessage
                            visible: text.length > 0
                            wrapMode: Text.WordWrap
                        }

                        SecondaryButton {
                            visible: backend.errorMessage.length > 0
                            text: "←  Review details"
                            onClicked: pages.currentIndex = 2
                        }

                        Item { Layout.fillHeight: true }
                    }
                }

                // Completion summary
                Item {
                    id: summaryPage
                    focus: pages.currentIndex === 4
                    KeyNavigation.tab: finishButton
                    KeyNavigation.backtab: finishButton
                    Keys.onReturnPressed: event => {
                        if (finishButton.enabled)
                            finishButton.clicked()
                        event.accepted = true
                    }
                    Keys.onEnterPressed: event => {
                        if (finishButton.enabled)
                            finishButton.clicked()
                        event.accepted = true
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(14 * window.uiScale)

                        PageHeading { text: "Your system is ready" }
                        PageDescription {
                            text: "Your account has been created successfully. Continue to the login screen to sign in to Fluff Linux."
                        }

                        Item { Layout.preferredHeight: Math.round(8 * window.uiScale) }

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: Math.round(142 * window.uiScale)
                            radius: 9
                            color: "white"
                            border.color: window.borderColor
                            border.width: 1

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: Math.round(19 * window.uiScale)
                                spacing: Math.round(13 * window.uiScale)

                                FieldLabel { text: "Setup summary" }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.preferredWidth: Math.round(125 * window.uiScale)
                                        text: "System name"
                                        color: window.bodyColor
                                        font.pixelSize: Math.round(14 * window.uiScale)
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: hostnameField.text.trim().length > 0
                                              ? hostnameField.text.trim()
                                              : hostnameField.placeholderText
                                        color: window.headingColor
                                        font.pixelSize: Math.round(15 * window.uiScale)
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.preferredWidth: Math.round(125 * window.uiScale)
                                        text: "Name"
                                        color: window.bodyColor
                                        font.pixelSize: Math.round(14 * window.uiScale)
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: nameField.text.trim().length > 0
                                              ? nameField.text
                                              : nameField.placeholderText
                                        color: window.headingColor
                                        font.pixelSize: Math.round(15 * window.uiScale)
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                }
                            }
                        }

                        Item { Layout.fillHeight: true }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "✓"
                            color: "#2f9e44"
                            font.pixelSize: Math.round(76 * window.uiScale)
                            font.weight: Font.DemiBold
                        }

                        ErrorText {
                            Layout.fillWidth: true
                            text: backend.errorMessage
                            visible: text.length > 0
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Item { Layout.fillHeight: true }

                        PrimaryButton {
                            id: finishButton
                            Layout.alignment: Qt.AlignRight
                            text: "Finish"
                            enabled: !backend.busy
                            onClicked: Qt.quit()
                        }
                    }
                }
            }
        }
    }

    // Shared controls keep scaling, spacing, and keyboard focus consistent on
    // every page without depending on a particular Plasma theme.
    component PageHeading: Text {
        Layout.fillWidth: true
        color: window.headingColor
        font.pixelSize: Math.round(29 * window.uiScale)
        font.weight: Font.DemiBold
        wrapMode: Text.WordWrap
    }

    component PageDescription: Text {
        Layout.fillWidth: true
        color: window.bodyColor
        font.pixelSize: Math.round(15 * window.uiScale)
        lineHeight: 1.2
        wrapMode: Text.WordWrap
    }

    component FieldLabel: Text {
        color: "#222222"
        font.pixelSize: Math.round(14 * window.uiScale)
        font.weight: Font.DemiBold
    }

    component ErrorText: Text {
        Layout.fillWidth: true
        color: "#9b1111"
        font.pixelSize: Math.round(13 * window.uiScale)
        wrapMode: Text.WordWrap
    }

    component InformationIcon: Item {
        implicitWidth: Math.round(24 * window.uiScale)
        implicitHeight: Math.round(24 * window.uiScale)

        Rectangle {
            anchors.centerIn: parent
            width: Math.round(20 * window.uiScale)
            height: width
            radius: width / 2
            color: "transparent"
            border.width: Math.max(2, Math.round(2 * window.uiScale))
            border.color: "white"
        }

        Text {
            anchors.centerIn: parent
            text: "i"
            color: "white"
            font.pixelSize: Math.round(15 * window.uiScale)
            font.weight: Font.Bold
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignHCenter
        }
    }

    component PasswordEyeIcon: Canvas {
        property bool opened: true
        implicitWidth: Math.round(24 * window.uiScale)
        implicitHeight: Math.round(18 * window.uiScale)

        onOpenedChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()

        onPaint: {
            const context = getContext("2d")
            const centerX = width / 2
            const centerY = height / 2
            const inset = Math.max(1.5, 2 * window.uiScale)

            context.clearRect(0, 0, width, height)
            context.strokeStyle = "white"
            context.fillStyle = "white"
            context.lineWidth = Math.max(1.5, 2 * window.uiScale)
            context.lineCap = "round"
            context.lineJoin = "round"

            context.beginPath()
            context.moveTo(inset, centerY)
            context.bezierCurveTo(width * 0.25, inset,
                                  width * 0.75, inset,
                                  width - inset, centerY)
            context.bezierCurveTo(width * 0.75, height - inset,
                                  width * 0.25, height - inset,
                                  inset, centerY)
            context.stroke()

            context.beginPath()
            context.arc(centerX, centerY,
                        Math.max(2, Math.min(width, height) * 0.16),
                        0, Math.PI * 2)
            context.fill()

            if (!opened) {
                context.beginPath()
                context.moveTo(inset, height - inset)
                context.lineTo(width - inset, inset)
                context.stroke()
            }
        }
    }

    component SetupField: TextField {
        implicitHeight: Math.round(44 * window.uiScale)
        activeFocusOnTab: true
        color: window.headingColor
        font.pixelSize: Math.round(15 * window.uiScale)
        selectByMouse: true
        selectionColor: window.accent
        selectedTextColor: "white"
        placeholderTextColor: "#81848a"
        leftPadding: Math.round(12 * window.uiScale)
        rightPadding: Math.round(12 * window.uiScale)
        background: Rectangle {
            radius: 7
            color: "white"
            border.width: parent.activeFocus ? 3 : 1
            border.color: parent.activeFocus ? window.headingColor : window.borderColor
        }
    }

    component PrimaryButton: Button {
        id: control
        activeFocusOnTab: true
        implicitWidth: Math.round(142 * window.uiScale)
        implicitHeight: Math.round(44 * window.uiScale)
        font.pixelSize: Math.round(15 * window.uiScale)
        font.weight: Font.DemiBold
        Keys.onReturnPressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        Keys.onEnterPressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        Keys.onSpacePressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        contentItem: Text {
            text: control.text
            color: "white"
            font: control.font
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Item {
            Rectangle {
                anchors.fill: parent
                anchors.margins: control.visualFocus ? -4 : 0
                radius: 11
                color: "transparent"
                border.width: control.visualFocus ? 4 : 0
                border.color: window.headingColor
            }

            Rectangle {
                anchors.fill: parent
                radius: 7
                color: !control.enabled ? "#a78686"
                      : control.down ? "#680000"
                      : control.hovered ? "#970606"
                      : window.accent
                border.width: 1
                border.color: !control.enabled ? "#a78686" : "#690000"
            }
        }
    }

    component SecondaryButton: Button {
        id: control
        activeFocusOnTab: true
        implicitWidth: Math.round(142 * window.uiScale)
        implicitHeight: Math.round(44 * window.uiScale)
        font.pixelSize: Math.round(15 * window.uiScale)
        font.weight: Font.DemiBold
        Keys.onReturnPressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        Keys.onEnterPressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        Keys.onSpacePressed: event => {
            if (control.enabled)
                control.clicked()
            event.accepted = true
        }
        contentItem: Text {
            text: control.text
            color: "#2d2d30"
            font: control.font
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: 7
            color: control.down ? "#e2e4e7" : control.hovered ? "#eceef1" : "white"
            border.width: control.visualFocus ? 3 : 1
            border.color: control.visualFocus ? window.headingColor : window.borderColor
        }
    }

}
