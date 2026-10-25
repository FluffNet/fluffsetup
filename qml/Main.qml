import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.flufflinux.setup

ApplicationWindow {
    id: window
    function tr(key) {
        // Invokable results alone do not notify QML when the language changes.
        const language = backend.selectedLanguage
        return backend.translate(key)
    }
    readonly property bool rtl: backend.rightToLeft
    readonly property string baseLanguage: backend.baseLanguage(backend.selectedLanguage)
    readonly property var regionChoices: JSON.parse(backend.regionList(backend.selectedLanguage))
    readonly property bool languageSelectionsValid: languagePicker.hasSelection
        && (regionChoices.length <= 1 || regionPicker.hasSelection) && countryPicker.hasSelection
    function continueLanguage() {
        if (!languagePicker.hasSelection) return
        if (regionChoices.length > 1) showPage(6, regionPicker)
        else showPage(1, welcomeNextButton)
    }
    function continueRegionalLanguage() {
        if (regionPicker.hasSelection) showPage(1, welcomeNextButton)
    }
    readonly property string nextText: rtl ? "←  " + tr("next") : tr("next") + "  →"
    readonly property string backArrow: rtl ? "→" : "←"
    readonly property string backText: backArrow + "  " + tr("back")
    readonly property int logicalScreenWidth: backend.preview ? backend.previewWidth : screen.width
    readonly property int logicalScreenHeight: backend.preview ? backend.previewHeight : screen.height
    readonly property int bodyAlignment: rtl ? Text.AlignRight : Text.AlignLeft
    readonly property bool hasTypingInput: [2, 3, 7, 9].includes(pages.currentIndex)
    readonly property var selectedKeyboardLayouts: {
        const all = JSON.parse(backend.keyboardList(backend.selectedLanguage, backend.keyboardSelection, backend.selectedRegion))
        return backend.keyboardSelection.split(",").map(id => all.find(layout => layout.id === id)).filter(layout => layout !== undefined)
    }
    readonly property bool showKeyboardIndicator: hasTypingInput && selectedKeyboardLayouts.length > 1
    property int clockTick: 0
    readonly property var timezoneClock: {
        const tick = clockTick
        const zone = backend.selectedTimezone
        const language = backend.selectedLanguage
        const region = backend.selectedRegion
        const synchronized = backend.clockSynchronized
        return pages.currentIndex === 9 ? JSON.parse(backend.timezoneClock()) : {time: "", date: "", synchronized: false, source: ""}
    }
    property var lastTypingField: null
    onActiveFocusItemChanged: {
        if ([hostnameField, nameField, passwordField, confirmPasswordField, keyboardPicker.searchInput, timezonePicker.searchInput].includes(activeFocusItem))
            lastTypingField = activeFocusItem
    }
    function restoreTypingFocus() {
        const field = lastTypingField
        const matchesPage = pages.currentIndex === 2 && field === hostnameField
                || pages.currentIndex === 3 && [nameField, passwordField, confirmPasswordField].includes(field)
                || pages.currentIndex === 7 && field === keyboardPicker.searchInput
                || pages.currentIndex === 9 && field === timezonePicker.searchInput
        if (matchesPage && field.enabled) field.forceActiveFocus()
        else if (hasTypingInput) restoreFocus()
    }
    // Theme icons are preferred, but a missing theme must not leave a blank panel.
    readonly property url userIconFallback: "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><g fill="none" stroke="#333333" stroke-width="3"><circle cx="32" cy="19" r="11"/><path d="M10 58v-7c0-12 10-19 22-19s22 7 22 19v7z"/></g></svg>')
    readonly property url networkIconFallback: "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><g fill="none" stroke="#333333" stroke-width="3"><rect x="6" y="8" width="52" height="35" rx="2"/><path d="M6 35h52M32 43v12M18 55h28"/></g></svg>')
    // The native session layer assigns the primary output before showing this
    // window. All sizing stays bound to that assigned output, so changing the
    // screen later also recalculates and centers the setup window correctly.
    readonly property bool compactScreen: logicalScreenWidth <= 1280 || logicalScreenHeight <= 720
    readonly property int targetWidth: compactScreen
                                       ? logicalScreenWidth
                                       : Math.round(logicalScreenWidth * 0.90)
    readonly property int targetHeight: compactScreen
                                        ? logicalScreenHeight
                                        : Math.round(logicalScreenHeight * 0.88)
    readonly property real uiScale: Math.min(1.25,
                                             Math.min(width / 960,
                                                      height / 820))
    readonly property int pageHorizontalMargin: Math.round(62 * uiScale)
    readonly property int pageTopMargin: Math.round(68 * uiScale)
    readonly property int pageBottomMargin: Math.round(48 * uiScale)

    width: targetWidth
    height: targetHeight
    minimumWidth: targetWidth
    maximumWidth: targetWidth
    minimumHeight: targetHeight
    maximumHeight: targetHeight
    visible: backend.preview
    title: "Fluff Linux Setup"
    color: "transparent"
    modality: Qt.ApplicationModal
    flags: Qt.Dialog | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    x: screen.virtualX + Math.round((screen.width - width) / 2)
    y: screen.virtualY + Math.round((screen.height - height) / 2)

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
                : backend.defaultName()
    }

    function saveRecoveryPage(page) {
        if (backend.preview) return
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
            languagePicker.forceActiveFocus()
        else if (pages.currentIndex === 6)
            regionPicker.forceActiveFocus()
        else if (pages.currentIndex === 1)
            welcomePage.forceActiveFocus()
        else if (pages.currentIndex === 7)
            keyboardPicker.focusSearch()
        else if (pages.currentIndex === 8)
            countryPicker.forceActiveFocus()
        else if (pages.currentIndex === 2)
            hostnameField.forceActiveFocus()
        else if (pages.currentIndex === 3)
            nameField.forceActiveFocus()
        else if (pages.currentIndex === 9)
            timezonePicker.focusSearch()
        else if (pages.currentIndex === 5)
            summaryPage.forceActiveFocus()
    }

    Component.onCompleted: {
        pages.currentIndex = backend.resumePage()
        Qt.callLater(restoreFocus)
        if (backend.previewKeyboardMenu() && showKeyboardIndicator) {
            // Qt's native content root cannot be grabbed by QML. Include the
            // real popup overlay in the preview's capturable item instead.
            Overlay.overlay.parent = setupSurface
            Qt.callLater(function() { keyboardIndicator.clicked() })
        }
    }

    // Coalesce typing into small recovery updates instead of writing the state
    // file on every keystroke.
    Timer {
        id: recoverySaveTimer
        interval: 300
        repeat: false
        onTriggered: {
            if (pages.currentIndex === 2 || pages.currentIndex === 3 || pages.currentIndex === 9)
                window.saveRecoveryPage(pages.currentIndex)
        }
    }

    SetupBackend {
        id: backend
    }

    Timer {
        interval: 5000
        running: pages.currentIndex === 9 && window.visible
        triggeredOnStart: true
        repeat: true
        onTriggered: backend.refreshClockSync()
    }

    Timer {
        interval: 1000
        running: pages.currentIndex === 9 && backend.selectedTimezone.length > 0
        repeat: true
        onTriggered: ++window.clockTick
    }

    // Restore the saved layouts at startup and retry a temporarily unavailable
    // compositor. A successfully applied, unchanged selection is a cheap no-op.
    Timer {
        interval: 2000
        running: !backend.preview && !backend.busy && !backend.completed
        repeat: true
        triggeredOnStart: true
        onTriggered: backend.syncKeyboardSession()
    }

    // Read the compositor's group, not a guessed index. Status and switches run
    // off the GUI thread and do not rewrite the saved default or layout order.
    Timer {
        interval: 500
        running: window.showKeyboardIndicator && !backend.preview && backend.keyboardSessionReady
                 && !backend.busy && !backend.completed
        repeat: true
        triggeredOnStart: true
        onTriggered: backend.refreshKeyboardLayout()
    }

    // Render real pages without touching recovery state or privileged helpers.
    Timer {
        property bool timezoneExercised: false
        property var timezoneExerciseIssues: []
        property bool waitingForMap: false
        property int mapWaitTicks: 0
        interval: waitingForMap ? 100 : pages.currentIndex === 9 ? 2000 : 250
        running: backend.preview && backend.snapshotPath.length > 0
        onTriggered: {
            const issues = []
            if (backend.exercisePreview() && window.hasTypingInput) {
                const selection = backend.keyboardSelection
                const originalIndex = backend.activeKeyboardIndex
                const ids = selection.split(",")
                if (ids.length > 1) {
                    backend.switchKeyboardLayout(ids[1])
                    if (backend.activeKeyboardIndex !== 1) issues.push({indicator: "Menu switch did not update active group"})
                    backend.switchKeyboardLayout("not-selected")
                    if (backend.activeKeyboardIndex !== 1) issues.push({indicator: "Invalid layout switched active group"})
                    backend.switchKeyboardLayout(ids[originalIndex])
                }
                if (backend.keyboardSelection !== selection) issues.push({indicator: "Switch changed saved default/order"})
            }
            if (pages.currentIndex === 9 && backend.exercisePreview() && !timezoneExercised) {
                const originalLanguage = backend.selectedLanguage
                const originalRegion = backend.selectedRegion
                const originalZone = backend.selectedTimezone
                const originalKeyboards = backend.keyboardSelection
                backend.selectTimezone("Europe/London")
                backend.selectTimezone("../etc/passwd")
                if (backend.selectedTimezone !== "Europe/London")
                    issues.push({timezone: "Invalid timezone changed the selection"})
                backend.selectTimezone("UTC")
                if (backend.selectedTimezone !== "UTC")
                    issues.push({timezone: "Explicit UTC choice was rejected"})
                backend.selectTimezone("Europe/London")
                backend.selectRegion("JP")
                backend.selectLanguage("he")
                if (backend.selectedTimezone !== "Europe/London")
                    issues.push({timezone: "Region/language erased a manual timezone"})
                for (let query of ["Jerusalem", "Israel", "ירושלים", "ישראל"]) {
                    timezonePicker.searchText = query
                    if (!timezonePicker.filteredZones.some(zone => zone.code === "Asia/Jerusalem"))
                        issues.push({timezoneSearch: query})
                }
                timezonePicker.searchText = "zzzzzzzzzz"
                if (timezonePicker.filteredZones.length) issues.push({timezone: "Empty search matched a zone"})
                if (timezoneNextButton.enabled) issues.push({timezone: "No-match search enabled Next"})
                timezoneNextButton.clicked()
                timezonePicker.searchInput.accepted()
                if (pages.currentIndex !== 9 || backend.selectedTimezone !== "Europe/London")
                    issues.push({timezone: "No-match search changed the page or timezone"})
                for (let action of ["next", "enter", "results"]) {
                    backend.selectedTimezone = ""
                    timezonePicker.searchText = "London"
                    if (!timezoneNextButton.enabled) issues.push({timezone: "Matching search disabled Next"})
                    if (action === "next") timezoneNextButton.clicked()
                    else if (action === "enter") timezonePicker.searchInput.accepted()
                    else timezonePicker.resultView.continueRequested()
                    if (pages.currentIndex !== 3 || backend.selectedTimezone !== "Europe/London")
                        issues.push({timezone: "Confirmation paths disagree", action})
                    window.showPage(9, timezonePicker.searchInput)
                }
                backend.selectLanguage(originalLanguage)
                backend.selectRegion(originalRegion)
                backend.keyboardSelection = originalKeyboards
                backend.selectedTimezone = originalZone
                timezonePicker.searchText = backend.previewTimezoneSearch()
                timezoneExercised = true
                // Preserve failures across the settling interval before capture.
                timezoneExerciseIssues = issues
                restart()
                return
            }
            if (timezoneExercised) issues.push(...timezoneExerciseIssues)
            if (pages.currentIndex === 8 && backend.exercisePreview()) {
                const originalLanguage = backend.selectedLanguage
                const originalRegion = backend.selectedRegion
                const originalSelection = backend.keyboardSelection
                backend.selectRegion("PS")
                if (backend.selectedRegion !== originalRegion) issues.push({interaction: "Excluded region was selectable"})
                backend.selectRegion("DE")
                if (backend.selectedLanguage !== originalLanguage || backend.selectedRegion !== "DE")
                    issues.push({interaction: "Geography changed the interface locale"})
                if (backend.keyboardSelection.split(",")[0] !== "de") issues.push({interaction: "Region did not set German defaults"})
                const germanDefaults = backend.keyboardSelection.split(",")
                // Reordering an existing group also works when defaults already
                // contain Hebrew or fill all four XKB groups.
                if (germanDefaults.length > 1) backend.defaultKeyboard(germanDefaults[germanDefaults.length - 1])
                else backend.addKeyboard("il")
                const custom = backend.keyboardSelection
                backend.selectRegion("FR")
                backend.selectLanguage("he")
                if (backend.keyboardSelection !== custom || backend.selectedRegion !== "FR")
                    issues.push({interaction: "Region/language erased customized keyboards or geography"})
                backend.selectLanguage(originalLanguage)
                backend.selectRegion(originalRegion)
                for (let id of backend.keyboardSelection.split(","))
                    if (!originalSelection.split(",").includes(id)) backend.removeKeyboard(id)
                backend.addKeyboard(originalSelection.split(",")[0])
                for (let id of backend.keyboardSelection.split(","))
                    if (!originalSelection.split(",").includes(id)) backend.removeKeyboard(id)
                for (let id of originalSelection.split(",")) backend.addKeyboard(id)
                for (let id of originalSelection.split(",").reverse()) backend.defaultKeyboard(id)
                if (backend.keyboardSelection !== originalSelection) issues.push({interaction: "Country audit did not restore keyboards"})
                countryNextButton.clicked()
                if (pages.currentIndex !== 7) issues.push({interaction: "Country Enter/Next did not reach keyboard"})
                pages.currentIndex = 8
                countryBackButton.clicked()
                if (pages.currentIndex !== 1) issues.push({interaction: "Country Back did not reach welcome"})
                pages.currentIndex = 8
            }
            if (pages.currentIndex === 7 && backend.exercisePreview()) {
                const originalLanguage = backend.selectedLanguage
                const originalSelection = backend.keyboardSelection
                pages.currentIndex = 1
                welcomeNextButton.clicked()
                if (pages.currentIndex !== 8) issues.push({interaction: "Welcome did not reach geographic region page"})
                countryNextButton.clicked()
                if (pages.currentIndex !== 7) issues.push({interaction: "Region did not reach keyboard page"})
                keyboardNextButton.clicked()
                if (pages.currentIndex !== 2) issues.push({interaction: "Keyboard did not reach hostname page"})
                hostnameBackButton.clicked()
                if (pages.currentIndex !== 7) issues.push({interaction: "Hostname Back skipped keyboard page"})
                let ids = backend.keyboardSelection.split(",")
                backend.addKeyboard(ids[0])
                if (backend.keyboardSelection !== originalSelection) issues.push({interaction: "Duplicate keyboard allowed"})
                if (ids.length > 1) {
                    backend.moveKeyboard(ids[1], -1)
                    const swapped = [ids[1], ids[0]].concat(ids.slice(2))
                    if (backend.keyboardSelection !== swapped.join(",")) issues.push({interaction: "Keyboard move up failed"})
                    backend.moveKeyboard(ids[1], 1)
                    if (backend.keyboardSelection !== originalSelection) issues.push({interaction: "Keyboard move down failed"})
                    backend.moveKeyboard(ids[0], -1)
                    if (backend.keyboardSelection !== originalSelection) issues.push({interaction: "Keyboard moved past first row"})
                    backend.defaultKeyboard(ids[1])
                    if (backend.keyboardSelection.split(",")[0] !== ids[1]) issues.push({interaction: "Default keyboard not reordered"})
                } else backend.addKeyboard("il")
                const dragIds = backend.keyboardSelection.split(",")
                const lastId = dragIds[dragIds.length - 1]
                backend.reorderKeyboard(lastId, 0)
                const dragOrder = [lastId].concat(dragIds.slice(0, -1)).join(",")
                if (backend.keyboardSelection !== dragOrder) issues.push({interaction: "Drag reorder did not make the first layout default"})
                backend.reorderKeyboard(lastId, dragIds.length)
                if (backend.keyboardSelection !== dragOrder) issues.push({interaction: "Out-of-bounds drag reordered keyboards"})
                const customized = backend.keyboardSelection
                backend.selectLanguage(originalLanguage === "fr" ? "he" : "fr")
                if (backend.keyboardSelection !== customized) issues.push({interaction: "Language change erased custom keyboard choices"})
                for (let id of ["us", "il", "de", "fr", "gb"]) backend.addKeyboard(id)
                if (backend.keyboardSelection.split(",").length > 4) issues.push({interaction: "Keyboard limit exceeded"})
                for (let id of backend.keyboardSelection.split(",")) backend.removeKeyboard(id)
                if (backend.keyboardSelection.split(",").length !== 1) issues.push({interaction: "Last keyboard could be removed"})
                backend.keyboardSelection = originalSelection
                backend.selectLanguage(originalLanguage)
                pages.currentIndex = 7
            }
            if (pages.currentIndex === 0 && backend.exercisePreview()) {
                const original = backend.selectedLanguage
                const originalBase = window.baseLanguage
                for (let choice of [["he", "שפה", true], ["fr", "Langue", false],
                                    ["ar", "اللغة", true], ["de", "Sprache", false]]) {
                    const index = languagePicker.languages.findIndex(language => language.code === choice[0])
                    languagePicker.choose(index)
                    if (backend.selectedLanguage !== choice[0] || languageHeading.text !== choice[1]
                            || window.rtl !== choice[2] || languagePicker.rtl !== choice[2]
                            || welcomeHeading.text !== backend.translate("welcome")
                            || hostnamePage.validationError !== backend.validateHostname(hostnameField.text))
                        issues.push({language: choice[0], interaction: "Live language binding did not update",
                                     selected: backend.selectedLanguage, heading: languageHeading.text,
                                     rtl: window.rtl, pickerRtl: languagePicker.rtl,
                                     welcome: welcomeHeading.text, expectedWelcome: backend.translate("welcome"),
                                     validation: hostnamePage.validationError,
                                     expectedValidation: backend.validateHostname(hostnameField.text)})
                }
                languagePicker.choose(languagePicker.languages.findIndex(language => language.code === originalBase))
                backend.selectLanguage(original)
                languagePicker.continueRequested()
                if (pages.currentIndex !== (window.regionChoices.length > 1 ? 6 : 1) || backend.selectedLanguage !== original)
                    issues.push({interaction: "Language selection did not continue to region/welcome"})
                pages.currentIndex = 0
            }
            function audit(item) {
                // KDE owns its clipped/scaled map controls. Audit our frame,
                // not the framework's separate logical coordinate space.
                if (item.objectName === "kde-timezone-map") return
                if (!item.visible) return
                // Recycled list rows outside the clipped viewport are not
                // visible UI. Do not mistake normal scrolling for clipping.
                for (let ancestor = item.parent; ancestor; ancestor = ancestor.parent) {
                    if (ancestor.clip) {
                        const local = item.mapToItem(ancestor, 0, 0)
                        if (local.y + item.height <= 0 || local.y >= ancestor.height
                                || local.x + item.width <= 0 || local.x >= ancestor.width) return
                    }
                }
                if (item.source !== undefined && item.status !== undefined
                        && item.source.toString().length > 0 && item.status !== Image.Ready)
                    issues.push({image: item.source.toString(), status: item.status})
                if (item.text && item.paintedHeight !== undefined) {
                    const position = item.mapToItem(setupSurface, 0, 0)
                    if (item.paintedHeight > item.height + 3 || item.paintedWidth > item.width + 3
                            || position.x < -3 || position.y < -3
                            || position.x + item.width > setupSurface.width + 3
                            || position.y + item.height > setupSurface.height + 3)
                        issues.push({text: item.text, x: position.x, y: position.y,
                                    width: item.width, height: item.height,
                                    paintedWidth: item.paintedWidth, paintedHeight: item.paintedHeight})
                }
                for (let child of item.children) audit(child)
            }
            if (pages.currentIndex === 9 && Qt.platform.os !== "osx") {
                const adapter = timezonePicker.mapItem
                if (!adapter || !adapter.mapItem || !adapter.mapItem.mapReady || adapter.fitting
                        || adapter.fittedZone !== backend.selectedTimezone) {
                    if (mapWaitTicks++ < 50) {
                        waitingForMap = true
                        restart()
                        return
                    }
                    issues.push({timezone: "Native map did not settle before capture"})
                }
            }
            if (!setupSurface.grabToImage(function(result) {
                // The captured frame has completed layout after live language
                // changes. Audit that frame rather than pending layout geometry.
                audit(setupSurface)
                if (keyboardIndicator.visible !== ([2, 3, 7, 9].includes(pages.currentIndex) && backend.keyboardSelection.split(",").length > 1))
                    issues.push({indicator: "Indicator visibility does not match typing page"})
                if (keyboardIndicator.visible) {
                    const position = keyboardIndicator.mapToItem(brandingPanel, 0, 0)
                    if (position.x < 0 || position.y < 0
                            || position.x + keyboardIndicator.width > brandingPanel.width + 1
                            || position.y + keyboardIndicator.height > brandingPanel.height + 1)
                        issues.push({indicator: "Indicator extends beyond branding panel"})
                    if (!keyboardIndicator.activeLayout || keyboardIndicator.activeLayout.id !== backend.keyboardSelection.split(",")[backend.activeKeyboardIndex])
                        issues.push({indicator: "Indicator does not match active group"})
                    if (keyboardIndicator.menu.opened) {
                        for (let index = 0; index < keyboardIndicator.menu.count; ++index) {
                            const row = keyboardIndicator.menu.itemAt(index)
                            const p = row.mapToItem(setupSurface, 0, 0)
                            const sidebarPosition = row.mapToItem(brandingPanel, 0, 0)
                            if (sidebarPosition.x < -1 || sidebarPosition.x + row.width > brandingPanel.width + 1)
                                issues.push({indicator: "Layout menu extends beyond sidebar", index})
                            if (p.x < -1 || p.y < -1 || p.x + row.width > window.width + 1 || p.y + row.height > window.height + 1)
                                issues.push({indicator: "Layout menu is outside assigned window", index, objectName: row.objectName, x: p.x, y: p.y, w: row.width, h: row.height})
                        }
                    }
                }
                if ([0, 6, 8].includes(pages.currentIndex)) {
                    const picker = pages.currentIndex === 0 ? languagePicker : pages.currentIndex === 6 ? regionPicker : countryPicker
                    const overflow = picker.count > 0 && picker.contentHeight > picker.height + 1
                    if (picker.verticalScrollBar.visible !== overflow)
                        issues.push({list: "Scrollbar visibility disagrees with actual overflow"})
                    if (overflow && picker.verticalScrollBar.contentItem.opacity <= 0)
                        issues.push({list: "Overflowing list's scrollbar thumb is transparent"})
                    const button = pages.currentIndex === 0 ? languageNextButton : pages.currentIndex === 6 ? regionNextButton : countryNextButton
                    const row = picker.currentItem
                    if (!row) issues.push({list: "Missing selected row"})
                    else {
                        let visibleRows = 0
                        for (let index = 0; index < picker.count; ++index) {
                            const item = picker.itemAtIndex(index)
                            if (!item || !item.visible) continue
                            const position = item.mapToItem(picker, 0, 0)
                            if (position.y < picker.height && position.y + item.height > 0) ++visibleRows
                        }
                        if (picker.count > 1 && picker.height > 2 * row.height && visibleRows < 2)
                            issues.push({list: "Language rows are missing from the visible viewport", visibleRows})
                        const rowPosition = row.mapToItem(setupSurface, 0, 0)
                        const buttonPosition = button.mapToItem(setupSurface, 0, 0)
                        const rowEdge = window.rtl ? rowPosition.x : rowPosition.x + row.width
                        const buttonEdge = window.rtl ? buttonPosition.x : buttonPosition.x + button.width
                        if (Math.abs(rowEdge - buttonEdge) > 1)
                            issues.push({alignment: "Language Next button does not align with rows", rowEdge, buttonEdge})
                        const local = row.mapToItem(picker, 0, 0)
                        if (local.y < -1 || local.y + row.height > picker.height + 1)
                            issues.push({language: backend.selectedLanguage, selection: "Selected language is outside the viewport"})
                    }
                }
                if (pages.currentIndex === 5) {
                    for (let value of [summaryHostnameValue, summaryNameValue])
                        if (value.horizontalAlignment !== window.bodyAlignment)
                            issues.push({text: value.text, alignment: "Incorrect summary value alignment"})
                }
                if (pages.currentIndex === 7) {
                    const ids = keyboardPicker.selectedIds
                    if (ids.length < 1 || ids.length > 4 || new Set(ids).size !== ids.length)
                        issues.push({keyboard: "Invalid selected keyboard list"})
                    if (keyboardPicker.selectedLayouts.length !== ids.length)
                        issues.push({keyboard: "Selected layout missing from registry"})
                    if (keyboardPicker.resultsView.height < 60 * window.uiScale - 1)
                        issues.push({keyboard: "Keyboard results have no usable viewport"})
                    const view = keyboardPicker.resultsView
                    const overflow = view.count > 0 && view.contentHeight > view.height + 1
                    if (keyboardPicker.verticalScrollBar.visible !== overflow)
                        issues.push({keyboard: "Scrollbar visibility disagrees with actual overflow"})
                    if (overflow && keyboardPicker.verticalScrollBar.contentItem.opacity <= 0)
                        issues.push({keyboard: "Overflowing list's scrollbar thumb is transparent"})
                    const row = view.itemAtIndex(0)
                    const rowPosition = row ? row.mapToItem(setupSurface, 0, 0)
                        : view.mapToItem(setupSurface, window.rtl ? view.leftMargin : view.width - view.rightMargin, 0)
                    const buttonPosition = keyboardNextButton.mapToItem(setupSurface, 0, 0)
                    const rowEdge = row ? (window.rtl ? rowPosition.x : rowPosition.x + row.width) : rowPosition.x
                    const buttonEdge = window.rtl ? buttonPosition.x : buttonPosition.x + keyboardNextButton.width
                    if (Math.abs(rowEdge - buttonEdge) > 1)
                        issues.push({alignment: "Keyboard Next button does not align with rows", rowEdge, buttonEdge})
                }
                if (pages.currentIndex === 9) {
                    if (backend.selectedTimezone.length && window.timezoneClock.synchronized && (!window.timezoneClock.time.length || !window.timezoneClock.date.length))
                        issues.push({timezone: "Selected timezone has no clock/date preview"})
                    if (backend.selectedTimezone.length && window.timezoneClock.synchronized && window.clockTick < 1)
                        issues.push({timezone: "Clock refresh timer did not run"})
                    if (!backend.selectedTimezone.length && window.timezoneClock.time.length)
                        issues.push({timezone: "Clock guessed a timezone before selection"})
                    if (!window.timezoneClock.synchronized && (window.timezoneClock.time.length || window.timezoneClock.date.length))
                        issues.push({timezone: "Unconfirmed/offline clock is visible"})
                    if (timezonePicker.resultView.height < timezonePicker.minimumResultsHeight - 1)
                        issues.push({timezone: "Results cannot fit five entries"})
                    if (Qt.platform.os !== "osx" && (!timezonePicker.mapItem || timezonePicker.mapItem.highlightedZone !== backend.selectedTimezone))
                        issues.push({timezone: "Map and selected timezone are out of sync"})
                    if (timezonePicker.mapItem && timezonePicker.mapItem.areaBounds) {
                        const adapter = timezonePicker.mapItem
                        const map = adapter.mapItem
                        if (adapter.fitting || adapter.fittedZone !== backend.selectedTimezone)
                            issues.push({timezone: "Selected area framing has not settled"})
                        if (adapter.viewportWidth > adapter.width + 1 || adapter.viewportHeight > adapter.height + 1)
                            issues.push({timezone: "Map viewport exceeds its frame"})
                        // Check the actual camera target. Perth frames its
                        // Australian pieces while retaining Antarctic geometry.
                        for (const coordinate of adapter.selectedCoordinates(true)) {
                            const point = map.fromCoordinate(coordinate, false)
                            if (!Number.isFinite(point.x) || !Number.isFinite(point.y)
                                    || point.x < adapter.fitMargin - 1 || point.x > map.width - adapter.fitMargin + 1
                                    || point.y < adapter.fitMargin - 1 || point.y > map.height - adapter.fitMargin + 1) {
                                issues.push({timezone: "Selected area border is clipped"})
                                break
                            }
                        }
                    }
                    if (timezoneNextButton.enabled !== timezonePicker.canConfirm)
                        issues.push({timezone: "Next disagrees with search confirmation"})
                    if (String(timezonePicker.searchInput.color) !== "#111111"
                            || String(timezonePicker.searchInput.selectionColor) !== "#820101"
                            || String(timezonePicker.searchInput.selectedTextColor) !== "#ffffff")
                        issues.push({timezone: "Search colors depend on the desktop theme"})
                    if (timezonePicker.searchInput.horizontalAlignment !== window.bodyAlignment)
                        issues.push({timezone: "Incorrect search alignment"})
                    const view = timezonePicker.resultView
                    const bar = view.verticalScrollBar
                    const overflow = view.count > 0 && view.contentHeight > view.height + 1
                    if (bar.visible !== overflow)
                        issues.push({timezone: "Scrollbar shown without overflowing results"})
                    const listPosition = view.mapToItem(setupSurface, 0, 0)
                    const barPosition = bar.mapToItem(setupSurface, 0, 0)
                    if (bar.visible && (window.rtl
                            ? barPosition.x + bar.width > listPosition.x + 1
                            : barPosition.x < listPosition.x + view.width - 1))
                        issues.push({timezone: "Scrollbar overlaps the result rows"})
                    const nextPosition = timezoneNextButton.mapToItem(setupSurface, 0, 0)
                    const nextEdge = window.rtl ? nextPosition.x : nextPosition.x + timezoneNextButton.width
                    const rowEdge = window.rtl ? listPosition.x : listPosition.x + view.width
                    if (Math.abs(nextEdge - rowEdge) > 1)
                        issues.push({timezone: "Next button does not align with result rows"})
                    if (!timezonePicker.searchText.trim().length
                            && timezonePicker.filteredZones.length !== timezonePicker.zones.length)
                        issues.push({timezone: "Empty search hides part of the timezone catalog"})
                    const utc = timezonePicker.zones.findIndex(zone => zone.code === "UTC")
                    if (utc >= 0 && (utc !== timezonePicker.zones.length - 1 || timezonePicker.zones[utc].suggested))
                        issues.push({timezone: "UTC is not a final, non-suggested manual choice"})
                    if (timezonePicker.zones.some(zone => zone.code.startsWith("Etc/")))
                        issues.push({timezone: "Duplicate technical timezone offered in the picker"})
                }
                if (pages.currentIndex !== 9 && timezonePicker.mapItem !== null)
                    issues.push({timezone: "Native map loaded before the timezone page's first visit"})
                console.log("LAYOUT_AUDIT " + JSON.stringify({width: width, height: height,
                                rtl: window.rtl, page: pages.currentIndex,
                                language: backend.selectedLanguage,
                                region: backend.selectedRegion,
                                keyboards: backend.keyboardSelection,
                                countryCount: countryPicker.count,
                                timezone: backend.selectedTimezone,
                                timezoneResults: timezonePicker.filteredZones.length,
                                timezoneCatalogCount: timezonePicker.zones.length,
                                timezoneScrollbarVisible: timezonePicker.resultView.verticalScrollBar.visible,
                                pickerScrollbarVisible: pages.currentIndex === 0 ? languagePicker.verticalScrollBar.visible
                                    : pages.currentIndex === 6 ? regionPicker.verticalScrollBar.visible
                                    : pages.currentIndex === 8 ? countryPicker.verticalScrollBar.visible : null,
                                keyboardScrollbarVisible: pages.currentIndex === 7 ? keyboardPicker.verticalScrollBar.visible : null,
                                timezoneClock: window.timezoneClock,
                                clockTicks: window.clockTick,
                                mapLoaded: timezonePicker.mapItem !== null,
                                mapCoverageLimited: timezonePicker.mapItem ? timezonePicker.mapItem.polarCoverageLimited : false,
                                mapHighlight: timezonePicker.mapItem ? timezonePicker.mapItem.highlightedZone : "",
                                timezoneSearch: timezonePicker.searchText,
                                timezoneSearchColor: String(timezonePicker.searchInput.color),
                                timezoneCanConfirm: timezonePicker.canConfirm,
                                hostnameErrors: pages.currentIndex === 2 ? hostnamePage.validationError : "",
                                issues: issues}))
                if (!result.saveToFile(backend.snapshotPath))
                    console.error("Snapshot could not be saved:", backend.snapshotPath)
                Qt.quit()
            })) Qt.quit()
        }
    }

    Connections {
        target: backend

        function onCompletedChanged() {
            if (backend.completed) {
                pages.currentIndex = 5
                Qt.callLater(function() { summaryPage.forceActiveFocus() })
            }
        }
    }

    Rectangle {
        id: setupSurface
        anchors.fill: parent
        radius: 0
        color: "#f7f8fa"
        clip: true
        border.color: "#d6d8dc"
        border.width: 1
        LayoutMirroring.enabled: window.rtl
        LayoutMirroring.childrenInherit: true

        RowLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                id: brandingPanel
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
                            visible: ![0, 6, 2, 3, 7, 8, 9].includes(pages.currentIndex)
                            source: backend.logoSource
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                        }

                        ToolButton {
                            anchors.centerIn: parent
                            width: parent.width
                            height: parent.height
                            visible: pages.currentIndex === 2
                            display: AbstractButton.IconOnly
                            icon.name: "network-wired"
                            icon.source: backend.previewIcons.length > 0 ? backend.previewIcons + "/network-wired.svg" : window.networkIconFallback
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
                            visible: pages.currentIndex === 3
                            display: AbstractButton.IconOnly
                            icon.name: "user-identity"
                            icon.source: backend.previewIcons.length > 0 ? backend.previewIcons + "/user-identity.svg" : window.userIconFallback
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
                            visible: pages.currentIndex === 0 || pages.currentIndex === 6
                            display: AbstractButton.IconOnly
                            icon.name: "translate"
                            icon.source: "qrc:/qt/qml/org/flufflinux/setup/assets/translate.svg"
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
                            visible: pages.currentIndex === 7
                            display: AbstractButton.IconOnly
                            icon.name: "input-keyboard"
                            icon.source: "qrc:/qt/qml/org/flufflinux/setup/assets/input-keyboard.svg"
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
                            visible: pages.currentIndex === 8
                            display: AbstractButton.IconOnly
                            icon.name: "globe"
                            icon.source: "qrc:/qt/qml/org/flufflinux/setup/assets/globe.svg"
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
                            visible: pages.currentIndex === 9
                            display: AbstractButton.IconOnly
                            icon.name: "preferences-system-time"
                            icon.source: "qrc:/qt/qml/org/flufflinux/setup/assets/preferences-system-time.svg"
                            icon.width: Math.round(148 * window.uiScale)
                            icon.height: Math.round(148 * window.uiScale)
                            hoverEnabled: false
                            focusPolicy: Qt.NoFocus
                            background: Item {}
                        }
                    }

                    Text {
                        objectName: "timezone-clock-time"
                        Layout.fillWidth: true
                        visible: pages.currentIndex === 9 && window.timezoneClock.time.length > 0
                        text: window.timezoneClock.time
                        color: window.headingColor
                        font.pixelSize: Math.round(24 * window.uiScale)
                        font.weight: Font.DemiBold
                        wrapMode: Text.Wrap
                        horizontalAlignment: Text.AlignHCenter
                        LayoutMirroring.enabled: false
                    }
                    Text {
                        objectName: "timezone-clock-date"
                        Layout.fillWidth: true
                        visible: pages.currentIndex === 9 && window.timezoneClock.date.length > 0
                        text: window.timezoneClock.date
                        color: window.bodyColor
                        font.pixelSize: Math.round(17 * window.uiScale)
                        wrapMode: Text.Wrap
                        horizontalAlignment: Text.AlignHCenter
                        LayoutMirroring.enabled: false
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        visible: ![0, 6, 2, 3, 7, 8, 9].includes(pages.currentIndex)
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
                    visible: pages.currentIndex === 1
                    text: "fluffsetup 1.5"
                    color: "#67686c"
                    font.pixelSize: Math.round(12 * window.uiScale)
                }
                KeyboardIndicator {
                    id: keyboardIndicator
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: Math.round(24 * window.uiScale)
                    width: Math.min(implicitWidth, parent.width - Math.round(48 * window.uiScale))
                    height: implicitHeight
                    visible: window.showKeyboardIndicator
                    enabled: backend.keyboardSessionReady && !backend.keyboardSwitching && !backend.busy
                    layouts: window.selectedKeyboardLayouts
                    activeIndex: backend.activeKeyboardIndex
                    rtl: window.rtl
                    uiScale: window.uiScale
                    popupAvailableWidth: brandingPanel.width
                    popupHorizontalOffset: -keyboardIndicator.x
                    popupAvailableHeight: window.height - keyboardIndicator.height - Math.round(48 * window.uiScale)
                    label: window.tr("keyboard")
                    onLayoutRequested: id => backend.switchKeyboardLayout(id)
                    onRestoreTypingFocus: Qt.callLater(window.restoreTypingFocus)
                }
            }

            StackLayout {
                id: pages
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: 0
                onCurrentIndexChanged: keyboardIndicator.menu.close()

                // Language previews immediately; only finalization applies the
                // chosen locale to the permanent user and operating system.
                Item {
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(18 * window.uiScale)
                        PageHeading { id: languageHeading; text: window.tr("language") }
                        LanguagePicker {
                            id: languagePicker
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            languages: JSON.parse(backend.languageList())
                            selectedCode: window.baseLanguage
                            rtl: window.rtl
                            uiScale: window.uiScale
                            KeyNavigation.tab: languageNextButton
                            onLanguageSelected: code => {
                                if (window.baseLanguage !== code) backend.selectLanguage(code)
                                window.saveRecoveryPage(0)
                            }
                            onContinueRequested: window.continueLanguage()
                        }
                        PrimaryButton {
                            id: languageNextButton
                            enabled: languagePicker.hasSelection
                            Layout.alignment: Qt.AlignRight
                            // Layout mirroring moves this logical trailing
                            // gutter to the scrollbar's physical side in RTL.
                            Layout.rightMargin: languagePicker.leftMargin + languagePicker.rightMargin
                            text: window.nextText
                            KeyNavigation.tab: languagePicker
                            onClicked: window.continueLanguage()
                        }
                    }
                }

                // Welcome
                Item {
                    id: welcomePage
                    focus: pages.currentIndex === 1
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
                            id: welcomeHeading
                            text: window.tr("welcome")
                            font.pixelSize: Math.round(36 * window.uiScale)
                            horizontalAlignment: Text.AlignHCenter
                        }
                        PageDescription {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.maximumWidth: Math.round(680 * window.uiScale)
                            text: window.tr("welcomeBody")
                            color: window.headingColor
                            font.pixelSize: Math.round(20 * window.uiScale)
                            lineHeight: 1.35
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true
                            SecondaryButton {
                                text: window.backText
                                onClicked: window.regionChoices.length > 1
                                           ? window.showPage(6, regionPicker) : window.showPage(0, languagePicker)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: welcomeNextButton
                                text: window.nextText
                                onClicked: window.showPage(8, countryPicker)
                            }
                        }
                    }
                }

                // System name
                Item {
                    id: hostnamePage
                    property string validationError: {
                        const language = backend.selectedLanguage
                        return backend.validateHostname(hostnameField.text)
                    }

                    function continueForward() {
                        if (validationError.length === 0)
                            window.showPage(9, timezonePicker.searchInput)
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
                                text: window.tr("systemTitle")
                                font.pixelSize: Math.round(36 * window.uiScale)
                                horizontalAlignment: Text.AlignHCenter
                            }
                            PageDescription {
                                text: window.tr("systemBody")
                                color: window.headingColor
                                font.pixelSize: Math.round(20 * window.uiScale)
                                lineHeight: 1.35
                                horizontalAlignment: Text.AlignHCenter
                            }
                            Item { Layout.preferredHeight: Math.round(8 * window.uiScale) }
                            FieldLabel { text: window.tr("systemName") }

                            SetupField {
                                id: hostnameField
                                LayoutMirroring.enabled: false
                                horizontalAlignment: TextInput.AlignLeft
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
                                text: window.tr("systemExamples")
                                LayoutMirroring.enabled: false
                                color: window.headingColor
                                font.pixelSize: Math.round(15 * window.uiScale)
                                wrapMode: Text.Wrap
                                horizontalAlignment: window.bodyAlignment
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
                                text: window.backText
                                KeyNavigation.tab: hostnameField
                                onClicked: window.showPage(7, keyboardPicker.searchInput)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: hostnameNextButton
                                text: window.nextText
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
                    property string nameError: {
                        const language = backend.selectedLanguage
                        return backend.validateName(nameField.text)
                    }
                    property string visibleError: {
                        if (nameError.length > 0)
                            return nameError
                        if (confirmPasswordField.text.length > 0
                                && passwordField.text !== confirmPasswordField.text)
                            return window.tr("mismatch")
                        return ""
                    }
                    property bool canContinue: window.languageSelectionsValid && nameError.length === 0
                                               && passwordField.text.length > 0
                                               && confirmPasswordField.text.length > 0
                                               && passwordField.text === confirmPasswordField.text

                    function continueForward() {
                        if (!canContinue || backend.busy)
                            return

                        window.saveRecoveryPage(3)
                        pages.currentIndex = 4
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

                        PageHeading { text: window.tr("accountTitle") }
                        PageDescription {
                            text: window.tr("accountBody")
                        }
                        FieldLabel { text: window.tr("name") }

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
                                    LayoutMirroring.enabled: false
                                    Layout.fillWidth: true
                                    Layout.minimumWidth: 0
                                    text: window.tr("notice")
                                    color: "white"
                                    font.pixelSize: Math.round(13 * window.uiScale)
                                    wrapMode: Text.WordWrap
                                    verticalAlignment: Text.AlignVCenter
                                    horizontalAlignment: window.bodyAlignment
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
                                          ? window.tr("hide")
                                          : window.tr("show")
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

                        FieldLabel { text: window.tr("password") }
                        SetupField {
                            id: passwordField
                            Layout.fillWidth: true
                            echoMode: accountPage.passwordsVisible
                                      ? TextInput.Normal
                                      : TextInput.Password
                            placeholderText: accountPage.passwordsVisible
                                             ? window.tr("visible")
                                             : window.tr("hidden")
                            KeyNavigation.tab: confirmPasswordField
                            onAccepted: accountPage.continueForward()
                        }

                        FieldLabel { text: window.tr("confirm") }
                        SetupField {
                            id: confirmPasswordField
                            Layout.fillWidth: true
                            echoMode: accountPage.passwordsVisible
                                      ? TextInput.Normal
                                      : TextInput.Password
                            placeholderText: window.tr("again")
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
                                text: window.backText
                                KeyNavigation.tab: nameField
                                onClicked: window.showPage(9, timezonePicker.searchInput)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: accountNextButton
                                text: window.nextText
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
                                  ? window.tr("errorTitle")
                                  : window.tr("applyingTitle")
                        }
                        PageDescription {
                            text: backend.errorMessage.length > 0
                                  ? window.tr("errorBody")
                                  : window.tr("applyingBody")
                        }

                        Item { Layout.fillHeight: true }

                        FieldLabel {
                            text: backend.errorMessage.length > 0
                                  ? window.tr("incomplete")
                                  : window.tr("applying")
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
                                            from: window.rtl ? movingChunk.parent.width : -movingChunk.width
                                            to: window.rtl ? -movingChunk.width : movingChunk.parent.width
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
                            text: window.backArrow + "  " + window.tr("review")
                            onClicked: window.showPage(3, nameField)
                        }

                        Item { Layout.fillHeight: true }
                    }
                }

                // Completion summary
                Item {
                    id: summaryPage
                    focus: pages.currentIndex === 5
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

                        PageHeading { text: window.tr("ready") }
                        PageDescription {
                            text: window.tr("readyBody")
                        }

                        Item { Layout.preferredHeight: Math.round(8 * window.uiScale) }

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: summaryContent.implicitHeight + 38 * window.uiScale
                            radius: 9
                            color: "white"
                            border.color: window.borderColor
                            border.width: 1

                            ColumnLayout {
                                id: summaryContent
                                anchors.fill: parent
                                anchors.margins: Math.round(19 * window.uiScale)
                                spacing: Math.round(13 * window.uiScale)

                                FieldLabel { text: window.tr("summary") }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.preferredWidth: Math.round(125 * window.uiScale)
                                        text: window.tr("systemName")
                                        LayoutMirroring.enabled: false
                                        color: window.bodyColor
                                        font.pixelSize: Math.round(14 * window.uiScale)
                                        wrapMode: Text.Wrap
                                        horizontalAlignment: window.bodyAlignment
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        id: summaryHostnameValue
                                        text: hostnameField.text.trim().length > 0
                                              ? hostnameField.text.trim()
                                              : hostnameField.placeholderText
                                        color: window.headingColor
                                        font.pixelSize: Math.round(15 * window.uiScale)
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                        LayoutMirroring.enabled: false
                                        horizontalAlignment: window.bodyAlignment
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.preferredWidth: Math.round(125 * window.uiScale)
                                        text: window.tr("name")
                                        LayoutMirroring.enabled: false
                                        color: window.bodyColor
                                        font.pixelSize: Math.round(14 * window.uiScale)
                                        wrapMode: Text.Wrap
                                        horizontalAlignment: window.bodyAlignment
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        id: summaryNameValue
                                        text: window.effectiveName()
                                        color: window.headingColor
                                        font.pixelSize: Math.round(15 * window.uiScale)
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                        LayoutMirroring.enabled: false
                                        horizontalAlignment: window.bodyAlignment
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
                            text: window.tr("finish")
                            enabled: !backend.busy && (backend.completed || backend.preview)
                            onClicked: Qt.quit()
                        }
                    }
                }
                // Keep existing page numbers stable for recovery records.
                // This page is reached after language selection, not completion.
                Item {
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(18 * window.uiScale)
                        PageHeading { text: window.tr("language") }
                        LanguagePicker {
                            id: regionPicker
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            languages: window.regionChoices
                            selectedCode: backend.selectedLanguage
                            rtl: window.rtl
                            uiScale: window.uiScale
                            KeyNavigation.tab: regionNextButton
                            onLanguageSelected: code => {
                                backend.selectLanguage(code)
                                window.saveRecoveryPage(6)
                            }
                            onContinueRequested: window.continueRegionalLanguage()
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.rightMargin: regionPicker.leftMargin + regionPicker.rightMargin
                            SecondaryButton {
                                text: window.backText
                                onClicked: window.showPage(0, languagePicker)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: regionNextButton
                                enabled: regionPicker.hasSelection
                                text: window.nextText
                                KeyNavigation.tab: regionPicker
                                onClicked: window.continueRegionalLanguage()
                            }
                        }
                    }
                }
                Item {
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(18 * window.uiScale)
                        PageHeading { text: window.tr("keyboard") }
                        KeyboardPicker {
                            id: keyboardPicker
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            layouts: JSON.parse(backend.keyboardList(backend.selectedLanguage, backend.keyboardSelection, backend.selectedRegion))
                            selectedIds: backend.keyboardSelection.split(",")
                            rtl: window.rtl
                            uiScale: window.uiScale
                            labels: ({selected: window.tr("keyboardSelected"), search: window.tr("keyboardSearchHint"),
                                      default: window.tr("keyboardDefault"), remove: window.tr("keyboardRemove"),
                                      add: window.tr("keyboardAdd"), empty: window.tr("keyboardEmpty"),
                                      other: window.tr("keyboardOther"),
                                      suggested: window.tr("keyboardSuggested"),
                                      moveUp: window.tr("keyboardMoveUp"), moveDown: window.tr("keyboardMoveDown"),
                                      limit: keyboardPicker.selectedIds.length + " / 4"})
                            Component.onCompleted: searchText = backend.previewKeyboardSearch
                            onAddRequested: id => { backend.addKeyboard(id); window.saveRecoveryPage(7) }
                            onRemoveRequested: id => { backend.removeKeyboard(id); window.saveRecoveryPage(7) }
                            onMoveRequested: (id, direction) => { backend.moveKeyboard(id, direction); window.saveRecoveryPage(7) }
                            onReorderRequested: (id, destination) => { backend.reorderKeyboard(id, destination); window.saveRecoveryPage(7) }
                            onContinueRequested: { if (keyboardNextButton.enabled) window.showPage(2, hostnameField) }
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: backend.keyboardSessionFailed
                            text: window.tr("keyboardSessionError")
                            color: window.accent
                            font.pixelSize: Math.round(15 * window.uiScale)
                            horizontalAlignment: window.bodyAlignment
                            LayoutMirroring.enabled: false
                            wrapMode: Text.Wrap
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            // Mirror the logical trailing scrollbar gutter,
                            // matching the language/region page's row alignment.
                            Layout.rightMargin: keyboardPicker.resultsView.leftMargin + keyboardPicker.resultsView.rightMargin
                            SecondaryButton {
                                text: window.backText
                                onClicked: window.showPage(8, countryPicker)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: keyboardNextButton
                                enabled: backend.preview || backend.keyboardSessionReady
                                text: window.nextText
                                KeyNavigation.tab: keyboardPicker.searchInput
                                onClicked: window.showPage(2, hostnameField)
                            }
                        }
                    }
                }
                // Geographic region is independent of the language's dialect.
                // Append page 8 so existing recovery page IDs stay unchanged.
                Item {
                    function continueWithRegion() {
                        if (!countryPicker.hasSelection) return
                        backend.selectRegion(backend.selectedRegion)
                        window.showPage(7, keyboardPicker.searchInput)
                    }
                    id: countryPage
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(18 * window.uiScale)
                        PageHeading { text: window.tr("regionTitle") }
                        LanguagePicker {
                            id: countryPicker
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            languages: JSON.parse(backend.countryList(backend.selectedLanguage))
                            selectedCode: backend.selectedRegion
                            labelLanguage: backend.selectedLanguage
                            grouped: true
                            rtl: window.rtl
                            uiScale: window.uiScale
                            KeyNavigation.tab: countryNextButton
                            onLanguageSelected: code => {
                                backend.selectRegion(code)
                                window.saveRecoveryPage(8)
                            }
                            onContinueRequested: countryPage.continueWithRegion()
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.rightMargin: countryPicker.leftMargin + countryPicker.rightMargin
                            SecondaryButton {
                                id: countryBackButton
                                text: window.backText
                                onClicked: window.showPage(1, welcomeNextButton)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: countryNextButton
                                enabled: countryPicker.hasSelection
                                text: window.nextText
                                KeyNavigation.tab: countryPicker
                                onClicked: countryPage.continueWithRegion()
                            }
                        }
                    }
                }
                // Append page 9 to preserve existing recovery page IDs.
                Item {
                    id: timezonePage
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: window.pageHorizontalMargin
                        anchors.rightMargin: window.pageHorizontalMargin
                        anchors.topMargin: window.pageTopMargin
                        anchors.bottomMargin: window.pageBottomMargin
                        spacing: Math.round(14 * window.uiScale)
                        PageHeading { text: window.tr("timezoneTitle") }
                        TimezonePicker {
                            id: timezonePicker
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            zones: JSON.parse(backend.timezoneList(backend.selectedLanguage, backend.selectedRegion))
                            selectedCode: backend.selectedTimezone
                            language: backend.selectedLanguage
                            region: backend.selectedRegion
                            rtl: window.rtl
                            uiScale: window.uiScale
                            searchPlaceholder: window.tr("timezoneSearch")
                            emptyText: window.tr("timezoneEmpty")
                            mapUnavailableText: window.tr("timezoneMapUnavailable")
                            mapText: (context, source) => backend.timezoneMapText(context, source)
                            Component.onCompleted: searchText = backend.previewTimezoneSearch()
                            onTypingFocused: window.lastTypingField = searchInput
                            onZoneSelected: id => {
                                backend.selectTimezone(id)
                                window.saveRecoveryPage(9)
                            }
                            onContinueRequested: { if (backend.selectedTimezone.length) window.showPage(3, nameField) }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.rightMargin: timezonePicker.resultsGutter
                            SecondaryButton {
                                id: timezoneBackButton
                                text: window.backText
                                onClicked: window.showPage(2, hostnameField)
                            }
                            Item { Layout.fillWidth: true }
                            PrimaryButton {
                                id: timezoneNextButton
                                text: window.nextText
                                enabled: timezonePicker.canConfirm
                                onClicked: timezonePicker.confirm()
                            }
                        }
                    }
                }
            }
        }
    }

    // Shared controls keep scaling, spacing, and keyboard focus consistent on
    // every page without depending on a particular Plasma theme.
    component PageHeading: Text {
        LayoutMirroring.enabled: false
        Layout.fillWidth: true
        color: window.headingColor
        font.pixelSize: Math.round(29 * window.uiScale)
        font.weight: Font.DemiBold
        wrapMode: Text.Wrap
        horizontalAlignment: window.bodyAlignment
    }

    component PageDescription: Text {
        LayoutMirroring.enabled: false
        Layout.fillWidth: true
        color: window.bodyColor
        font.pixelSize: Math.round(15 * window.uiScale)
        lineHeight: 1.2
        wrapMode: Text.Wrap
        horizontalAlignment: window.bodyAlignment
    }

    component FieldLabel: Text {
        LayoutMirroring.enabled: false
        Layout.fillWidth: true
        color: "#222222"
        font.pixelSize: Math.round(14 * window.uiScale)
        font.weight: Font.DemiBold
        wrapMode: Text.Wrap
        horizontalAlignment: window.bodyAlignment
    }

    component ErrorText: Text {
        LayoutMirroring.enabled: false
        textFormat: Text.PlainText
        Layout.fillWidth: true
        color: "#9b1111"
        font.pixelSize: Math.round(13 * window.uiScale)
        wrapMode: Text.Wrap
        horizontalAlignment: window.bodyAlignment
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
        id: field
        LayoutMirroring.enabled: false
        implicitHeight: Math.round(44 * window.uiScale)
        activeFocusOnTab: true
        color: window.headingColor
        font.pixelSize: Math.round(15 * window.uiScale)
        selectByMouse: true
        selectionColor: window.accent
        selectedTextColor: "white"
        // Fusion's stock placeholder does not honor the input alignment.
        // Keep its accessible value and paint the same placeholder explicitly.
        placeholderTextColor: "transparent"
        leftPadding: Math.round(12 * window.uiScale)
        rightPadding: Math.round(12 * window.uiScale)
        horizontalAlignment: window.rtl ? TextInput.AlignRight : TextInput.AlignLeft
        Text {
            anchors.fill: parent
            anchors.leftMargin: field.leftPadding
            anchors.rightMargin: field.rightPadding
            visible: field.length === 0 && field.preeditText.length === 0
            text: field.placeholderText
            font: field.font
            color: "#81848a"
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: field.horizontalAlignment
            LayoutMirroring.enabled: false
            elide: Text.ElideRight
        }
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
        Layout.maximumWidth: Math.max(1, (window.width - 424 * window.uiScale) / 2)
        implicitWidth: Math.max(142 * window.uiScale, buttonMetrics.advanceWidth + 32 * window.uiScale)
        implicitHeight: Math.max(44 * window.uiScale, contentItem.implicitHeight + 24 * window.uiScale)
        leftPadding: 16 * window.uiScale
        rightPadding: 16 * window.uiScale
        topPadding: 12 * window.uiScale
        bottomPadding: 12 * window.uiScale
        TextMetrics { id: buttonMetrics; font: control.font; text: control.text }
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
            wrapMode: Text.Wrap
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
        Layout.maximumWidth: Math.max(1, (window.width - 424 * window.uiScale) / 2)
        implicitWidth: Math.max(142 * window.uiScale, buttonMetrics.advanceWidth + 32 * window.uiScale)
        implicitHeight: Math.max(44 * window.uiScale, contentItem.implicitHeight + 24 * window.uiScale)
        leftPadding: 16 * window.uiScale
        rightPadding: 16 * window.uiScale
        topPadding: 12 * window.uiScale
        bottomPadding: 12 * window.uiScale
        TextMetrics { id: buttonMetrics; font: control.font; text: control.text }
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
            wrapMode: Text.Wrap
        }
        background: Rectangle {
            radius: 7
            color: control.down ? "#e2e4e7" : control.hovered ? "#eceef1" : "white"
            border.width: control.visualFocus ? 3 : 1
            border.color: control.visualFocus ? window.headingColor : window.borderColor
        }
    }

}
