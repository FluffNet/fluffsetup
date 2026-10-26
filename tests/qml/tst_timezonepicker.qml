import QtQuick
import QtQuick.Controls
import QtTest
import QtPositioning
import "../../qml"

Item {
    id: surface
    property bool rejectSelection: false
    width: 760
    height: 650
    TimezonePicker {
        id: picker
        anchors.fill: parent
        mapEnabled: false
        zones: [
            {code: "Asia/Jerusalem", label: "ירושלים (ישראל)", search: "Jerusalem Israel ירושלים ישראל Asia/Jerusalem"},
            {code: "Europe/London", label: "לונדון (בריטניה)", search: "London United Kingdom לונדון בריטניה Europe/London"},
            {code: "America/New_York", label: "ניו יורק (ארצות הברית)", search: "New York United States ניו יורק ארצות הברית America/New_York"},
            {code: "Europe/Paris", label: "Paris (France)", search: "Paris France פריז צרפת Europe/Paris"}
        ]
        selectedCode: "Asia/Jerusalem"
        onZoneSelected: id => { if (!surface.rejectSelection) selectedCode = id }
    }
    Button {
        id: nextButton
        x: 660
        y: 4
        width: 90
        height: 36
        text: "Next"
        enabled: picker.canConfirm
        onClicked: picker.confirm()
    }
    SignalSpy { id: selection; target: picker; signalName: "zoneSelected" }
    SignalSpy { id: continuation; target: picker; signalName: "continueRequested" }
    Component {
        id: unselectedPicker
        TimezonePicker {
            mapEnabled: false
            selectedCode: ""
            onZoneSelected: id => selectedCode = id
        }
    }
    TestCase {
        name: "TimezonePicker"
        when: windowShown
        function init() {
            surface.rejectSelection = false
            picker.searchText = ""
            picker.selectedCode = "Asia/Jerusalem"
            picker.region = ""
            picker.rtl = false
            picker.uiScale = 1
            selection.clear()
            continuation.clear()
            picker.focusSearch()
        }
        function test_first_map_selection_reveals_list_row_data() {
            const rows = []
            for (const rtl of [false, true]) for (const scale of [1, 0.55])
                rows.push({tag: (rtl ? "RTL" : "LTR") + "-" + scale, rtl, scale})
            return rows
        }
        function test_first_map_selection_reveals_list_row(data) {
            const rows = Array.from({length: 40}, (_, index) => ({
                code: index === 18 ? "America/New_York" : "zone-" + index,
                label: index === 18 ? "New York (United States)" : "Timezone " + index,
                english: "Timezone " + index
            }))
            const subject = createTemporaryObject(unselectedPicker, surface, {
                width: 760, height: 650, zones: rows, rtl: data.rtl, uiScale: data.scale
            })
            verify(subject)
            const view = subject.resultView
            tryCompare(view, "count", rows.length)
            wait(100)
            verify(view.atYBeginning)
            compare(subject.selectedCode, "")
            subject.zoneSelected("America/New_York")
            compare(subject.selectedCode, "America/New_York")
            compare(view.selectedIndex, 18)
            tryCompare(view, "currentIndex", 18)
            tryVerify(() => view.currentItem && view.currentItem.selected
                && view.currentItem.y >= view.contentY - 1
                && view.currentItem.y + view.currentItem.height <= view.contentY + view.height + 1,
                1000, "The first map selection must reveal its highlighted list row")
            // Filtering can temporarily remove the selected row. Restoring
            // the catalog or replacing its translated model must reveal it.
            subject.searchText = "no-such-city"
            tryCompare(view, "currentIndex", -1)
            subject.searchText = ""
            tryCompare(view, "currentIndex", 18)
            subject.zones = rows.slice().reverse()
            tryCompare(view, "currentIndex", 21)
            tryVerify(() => view.currentItem && view.currentItem.selected
                && view.currentItem.y >= view.contentY - 1
                && view.currentItem.y + view.currentItem.height <= view.contentY + view.height + 1)
        }
        function test_us_initial_view_keeps_all_timezone_choices() {
            const original = picker.zones
            picker.zones = [
                {code:"America/New_York",label:"New York",suggested:true,center:[40.7142,-74.0064]},
                {code:"America/Anchorage",label:"Anchorage",suggested:true,center:[61.2181,-149.9003]},
                {code:"Pacific/Honolulu",label:"Honolulu",suggested:true,center:[21.3069,-157.8583]}
            ]
            picker.region = "US"
            picker.selectedCode = ""
            verify(picker.initialMapBounds && picker.initialMapBounds.isValid)
            compare(picker.initialMapBounds.topLeft.latitude,50)
            compare(picker.initialMapBounds.bottomRight.longitude,-66)
            compare(picker.filteredZones.length,3)
            for (const code of ["America/New_York","America/Anchorage","Pacific/Honolulu"]) {
                picker.selectedCode = code
                compare(picker.initialMapBounds,null)
                compare(picker.selectedCenter,picker.zones.find(row => row.code === code).center)
            }
            picker.selectedCode = ""
            picker.region = "RU"
            compare(picker.initialMapBounds,null)
            picker.zones = original
        }
        function test_australia_initial_view_keeps_distant_island_choices() {
            const original = picker.zones
            picker.zones = [
                {code:"Australia/Perth",label:"Perth",suggested:true,center:[-31.95,115.85]},
                {code:"Australia/Hobart",label:"Hobart",suggested:true,center:[-42.8833,147.3167]},
                {code:"Australia/Lord_Howe",label:"Lord Howe",suggested:true,center:[-31.55,159.0833]},
                {code:"Antarctica/Macquarie",label:"Macquarie",suggested:true,center:[-54.5,158.95]}
            ]
            picker.region = "AU"
            picker.selectedCode = ""
            verify(picker.initialMapBounds && picker.initialMapBounds.isValid)
            compare(picker.initialMapBounds.topLeft.latitude,-9)
            compare(picker.initialMapBounds.topLeft.longitude,112)
            compare(picker.initialMapBounds.bottomRight.latitude,-44.5)
            compare(picker.initialMapBounds.bottomRight.longitude,160)
            compare(picker.filteredZones.length,4)
            verify(!picker.canConfirm)
            for (const zone of picker.zones) {
                picker.selectedCode = zone.code
                compare(picker.initialMapBounds,null)
                compare(picker.selectedCenter,zone.center)
            }
            picker.zones = original
        }
        function test_dateline_initial_center_and_new_zealand_bounds() {
            const original = picker.zones
            picker.zones = [
                {code:"Pacific/Auckland",label:"Auckland",suggested:true,center:[-36.8667,174.7667]},
                {code:"Pacific/Chatham",label:"Chatham",suggested:true,center:[-43.95,-176.55]}
            ]
            picker.region = "NZ"
            picker.selectedCode = ""
            verify(Math.abs(picker.selectedCenter[1]) > 175,"Dateline islands were centered on the opposite side of the world")
            verify(picker.initialMapBounds && picker.initialMapBounds.isValid)
            verify(picker.initialMapBounds.topLeft.longitude > picker.initialMapBounds.bottomRight.longitude)
            for (const zone of picker.zones) {
                verify(picker.initialMapBounds.contains(QtPositioning.coordinate(...zone.center)))
                picker.selectedCode = zone.code
                compare(picker.initialMapBounds,null)
                compare(picker.selectedCenter,zone.center)
                picker.selectedCode = ""
            }
            // The generic fallback must also wrap correctly outside the
            // explicitly framed region, and ignore unsuggested catalog rows.
            picker.region = "KI"
            compare(picker.initialMapBounds,null)
            picker.zones = picker.zones.concat({code:"Europe/London",label:"London",center:[51.5,0]})
            verify(Math.abs(picker.selectedCenter[1]) > 175)
            picker.zones = [
                {code:"east",suggested:true,center:[0,90]},
                {code:"west",suggested:true,center:[0,-90]}
            ]
            compare(picker.selectedCenter[1],90) // Deterministic antipodal fallback.
            picker.zones = original
        }
        function test_archipelago_and_chile_initial_bounds_data() {
            return [
                {tag:"Indonesia",region:"ID",zones:[
                    {code:"Asia/Jakarta",center:[-6.1667,106.8]},
                    {code:"Asia/Pontianak",center:[-0.0333,109.3333]},
                    {code:"Asia/Makassar",center:[-5.1167,119.4]},
                    {code:"Asia/Jayapura",center:[-2.5333,140.7]}
                ]},
                {tag:"Chile",region:"CL",zones:[
                    {code:"America/Santiago",center:[-33.45,-70.6667]},
                    {code:"America/Coyhaique",center:[-45.5667,-72.0667]},
                    {code:"America/Punta_Arenas",center:[-53.15,-70.9167]},
                    {code:"Pacific/Easter",center:[-27.15,-109.4333]}
                ]}
            ]
        }
        function test_archipelago_and_chile_initial_bounds(data) {
            const original = picker.zones
            picker.zones = data.zones.map(zone => Object.assign({suggested:true,label:zone.code},zone))
            picker.region = data.region
            picker.selectedCode = ""
            verify(picker.initialMapBounds && picker.initialMapBounds.isValid)
            verify(!picker.canConfirm)
            for (const zone of picker.zones) {
                verify(picker.initialMapBounds.contains(QtPositioning.coordinate(...zone.center)))
                picker.selectedCode = zone.code
                compare(picker.initialMapBounds,null)
                compare(picker.selectedCenter,zone.center)
                picker.selectedCode = ""
            }
            picker.zones = original
        }
        function test_regional_overview_ids_ignore_global_choices_and_search() {
            const original = picker.zones
            picker.zones = [
                {code:"Europe/Lisbon",label:"Lisbon",suggested:true},
                {code:"Atlantic/Azores",label:"Azores",suggested:true},
                {code:"Atlantic/Madeira",label:"Madeira",suggested:true},
                {code:"Europe/London",label:"London"},
                {code:"UTC",label:"UTC"}
            ]
            compare(picker.initialMapZones,["Europe/Lisbon","Atlantic/Azores","Atlantic/Madeira"])
            picker.searchText = "London"
            compare(picker.initialMapZones,["Europe/Lisbon","Atlantic/Azores","Atlantic/Madeira"])
            picker.selectedCode = "Europe/London"
            compare(picker.initialMapZones,["Europe/Lisbon","Atlantic/Azores","Atlantic/Madeira"])
            picker.zones = original
        }
        function test_confirmation_paths_data() {
            const cases = [
                {name:"empty",query:"",selected:"Asia/Jerusalem",expected:"Asia/Jerusalem"},
                {name:"spaces",query:"   ",selected:"Asia/Jerusalem",expected:"Asia/Jerusalem"},
                {name:"one",query:"London",selected:"Asia/Jerusalem",expected:"Europe/London"},
                {name:"native",query:"לוֹנְדוֹן",selected:"Asia/Jerusalem",expected:"Europe/London"},
                {name:"no-selection-match",query:"London",selected:"",expected:"Europe/London"},
                {name:"multiple-current",query:"Europe",selected:"Europe/Paris",expected:"Europe/Paris"},
                {name:"multiple-first",query:"Europe",selected:"Asia/Jerusalem",expected:"Europe/London"},
                {name:"none",query:"zzzz",selected:"Asia/Jerusalem",expected:""},
                {name:"none-no-selection",query:"zzzz",selected:"",expected:""},
                {name:"empty-no-selection",query:"",selected:"",expected:""}
            ]
            const rows = []
            for (const rtl of [false,true]) for (const action of ["next","return","enter","results-return","results-enter"])
                for (const item of cases) rows.push(Object.assign({}, item, {rtl,action,tag:(rtl?"RTL":"LTR")+"-"+action+"-"+item.name}))
            return rows
        }
        function test_confirmation_paths(data) {
            picker.rtl = data.rtl
            picker.selectedCode = data.selected
            picker.searchText = data.query
            compare(picker.confirmationCode, data.expected)
            compare(nextButton.enabled, data.expected.length > 0)
            if (data.action === "next") {
                if (nextButton.enabled) mouseClick(nextButton)
                else nextButton.clicked() // The shared handler must also reject direct calls.
            } else {
                if (data.action.startsWith("results")) picker.resultView.forceActiveFocus()
                else picker.focusSearch()
                keyClick(data.action.endsWith("return") ? Qt.Key_Return : Qt.Key_Enter)
            }
            compare(continuation.count, data.expected.length > 0 ? 1 : 0)
            compare(picker.selectedCode, data.expected.length ? data.expected : data.selected)
        }
        function test_rejected_selection_does_not_continue() {
            surface.rejectSelection = true
            picker.searchText = "London"
            picker.confirm()
            compare(selection.count, 1)
            compare(picker.selectedCode, "Asia/Jerusalem")
            compare(continuation.count, 0)
        }
        function test_touch_confirmation_and_clearing_no_matches_data() {
            return [{tag:"LTR",rtl:false},{tag:"RTL",rtl:true}]
        }
        function test_touch_confirmation_and_clearing_no_matches(data) {
            picker.rtl = data.rtl
            picker.searchText = "London"
            touchEvent(nextButton).press(0,nextButton).commit().release(0,nextButton).commit()
            compare(picker.selectedCode,"Europe/London")
            compare(continuation.count,1)
            picker.searchText = "zzzz"
            compare(picker.canConfirm,false)
            picker.searchText = ""
            compare(picker.canConfirm,true)
            compare(picker.selectedCode,"Europe/London")
        }
        function test_long_input_stays_inside_padding_data() {
            const rows = []
            for (const rtl of [false,true]) for (const scale of [1,0.55])
                rows.push({tag:(rtl?"RTL":"LTR")+"-"+scale,rtl,scale})
            return rows
        }
        function test_long_input_stays_inside_padding(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            const field = picker.searchInput
            for (const text of ["London ".repeat(600),"ירושלים 東京 ".repeat(400)]) {
                picker.searchText = text
                picker.focusSearch()
                field.cursorPosition = field.length
                wait(20)
                verify(field.cursorRectangle.x >= field.leftPadding - 1)
                verify(field.cursorRectangle.x + field.cursorRectangle.width <= field.width - field.rightPadding + 1)
                compare(field.color,"#111111")
                field.selectAll()
                compare(field.selectedText,text)
                keyClick(Qt.Key_Backspace)
                compare(picker.searchText,"")
            }
        }
        function test_text_colors_do_not_follow_palette() {
            const field = picker.searchInput
            for (const textColor of ["#fcfcfc","#111111"]) {
                field.palette.text = textColor
                field.palette.highlight = "#00ff00"
                field.palette.highlightedText = "#000000"
                picker.searchText = "London"
                compare(field.color, "#111111")
                compare(field.selectionColor, "#820101")
                compare(field.selectedTextColor, "#ffffff")
                field.selectAll()
                compare(field.selectedText, "London")
                field.deselect()
            }
        }
        function test_rapid_search_and_confirmation_stress() {
            const original = picker.zones
            const scenarios = [
                ["lOnDoN","Europe/London"], ["לוֹנְדוֹן","Europe/London"],
                ["東京","Asia/Tokyo"], ["日本","Asia/Tokyo"],
                ["São Paulo","America/Sao_Paulo"], ["Sao Paulo","America/Sao_Paulo"],
                ["Česko","Europe/Prague"], ["Praha","Europe/Prague"],
                ["กรุงเทพ","Asia/Bangkok"], ["zzzz",""], ["!".repeat(4096),""],
                [" \n\t ","Asia/Jerusalem"]
            ]
            picker.zones = original.concat([
                {code:"Asia/Tokyo",label:"東京 (日本)",english:"Tokyo",search:"Tokyo Japan 東京 日本"},
                {code:"America/Sao_Paulo",label:"São Paulo (Brasil)",english:"Sao Paulo",search:"São Paulo Brazil Brasil"},
                {code:"Europe/Prague",label:"Praha (Česko)",english:"Prague",search:"Prague Czechia Praha Česko"},
                {code:"Asia/Bangkok",label:"กรุงเทพ (ไทย)",english:"Bangkok",search:"Bangkok Thailand กรุงเทพ ไทย"}
            ])
            for (let iteration = 0; iteration < 600; ++iteration) {
                const scenario = scenarios[iteration % scenarios.length]
                picker.rtl = iteration % 2 === 0
                picker.uiScale = iteration % 3 === 0 ? 0.55 : 1
                picker.searchText = ""
                picker.selectedCode = "Asia/Jerusalem"
                picker.searchText = scenario[0]
                continuation.clear()
                picker.confirm()
                compare(continuation.count, scenario[1].length ? 1 : 0)
                compare(picker.selectedCode, scenario[1].length ? scenario[1] : "Asia/Jerusalem")
                compare(picker.searchInput.color, "#111111")
                if (iteration % 20 === 0) wait(1) // Exercise queued model/layout updates too.
            }
            picker.zones = original
        }
        function test_english_and_native_live_search() {
            for (let query of ["Jerusalem", "Israel", "ירושלים", "ישראל", "Asia/Jerusalem"]) {
                picker.searchText = query
                compare(picker.filteredZones.length, 1)
                compare(picker.filteredZones[0].code, "Asia/Jerusalem")
            }
            for (let query of ["new york", "United States", "ניו יורק", "ארצות הברית"]) {
                picker.searchText = query
                compare(picker.filteredZones.length, 1)
                compare(picker.filteredZones[0].code, "America/New_York")
            }
            picker.searchText = "zzzzzz"
            compare(picker.filteredZones.length, 0)
            picker.confirm()
            compare(continuation.count, 0)
            compare(picker.selectedCode, "Asia/Jerusalem")
        }
        function test_utc_search_and_confirmation_data() {
            return [{tag:"LTR", rtl:false}, {tag:"RTL", rtl:true}]
        }
        function test_utc_search_and_confirmation(data) {
            const original = picker.zones
            picker.rtl = data.rtl
            picker.zones = original.concat([{code:"UTC", label:"UTC", english:"UTC", search:"UTC", suggested:false}])
            for (const action of ["next", "return", "enter"]) {
                picker.selectedCode = "Asia/Jerusalem"
                picker.searchText = action === "return" ? "utc" : "UTC"
                compare(picker.filteredZones.length, 1)
                compare(picker.filteredZones[0].code, "UTC")
                compare(picker.selectedCode, "Asia/Jerusalem")
                continuation.clear()
                if (action === "next") mouseClick(nextButton)
                else {
                    picker.focusSearch()
                    keyClick(action === "return" ? Qt.Key_Return : Qt.Key_Enter)
                }
                compare(picker.selectedCode, "UTC")
                compare(continuation.count, 1)
                picker.searchText = ""
                compare(picker.filteredZones[picker.filteredZones.length - 1].code, "UTC")
                compare(picker.selectedCode, "UTC")
            }
            picker.zones = original
        }
        function test_enter_and_numpad_enter() {
            for (let key of [Qt.Key_Return, Qt.Key_Enter]) {
                picker.searchText = "London"
                picker.focusSearch()
                keyClick(key)
                compare(picker.selectedCode, "Europe/London")
                verify(continuation.count > 0)
            }
        }
        function test_keyboard_navigation_and_rtl_alignment() {
            picker.rtl = true
            compare(picker.searchInput.horizontalAlignment, Text.AlignRight)
            picker.searchText = "France"
            picker.focusSearch()
            keyClick(Qt.Key_Down)
            compare(picker.selectedCode, "Europe/Paris")
            verify(picker.resultView.activeFocus)
            keyClick(Qt.Key_Return)
            compare(continuation.count, 1)
        }
        function test_search_text_inset_data() {
            const rows = []
            for (const rtl of [false, true]) for (const scale of [1, 0.55])
                rows.push({tag: (rtl ? "RTL" : "LTR") + "-" + scale, rtl, scale})
            return rows
        }
        function test_search_text_inset(data) {
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            picker.focusSearch()
            const field = picker.searchInput
            const placeholder = findChild(field, "timezone-search-placeholder")
            verify(placeholder)
            compare(field.leftPadding, 7)
            compare(field.rightPadding, 7)
            compare(field.leftPadding - field.background.border.width, 5)
            tryCompare(placeholder, "x", field.leftPadding)
            tryVerify(() => Math.abs(placeholder.width - (field.width - field.leftPadding - field.rightPadding)) < 1)
            compare(placeholder.horizontalAlignment, data.rtl ? Text.AlignRight : Text.AlignLeft)
            verify(placeholder.visible)
            picker.searchText = "London"
            compare(placeholder.visible, false)
            compare(field.leftPadding, 7)
            compare(field.rightPadding, 7)
            picker.searchText = ""
            picker.resultView.forceActiveFocus()
            tryCompare(field, "activeFocus", false)
            compare(field.leftPadding - field.background.border.width, 6)
            verify(placeholder.visible)
        }
        function test_scaled_layout_and_empty_search_preserve_selection() {
            picker.uiScale = 0.55
            picker.searchText = "zzzz"
            compare(picker.selectedLabel, "ירושלים (ישראל)")
            verify(picker.searchInput.width <= picker.width)
            verify(picker.resultView.height >= 42 * picker.uiScale)
        }
        function test_no_selection_does_not_continue() {
            picker.selectedCode = ""
            picker.confirm()
            compare(continuation.count, 0)
        }
        function test_region_suggestions_and_global_search() {
            const original = picker.zones
            picker.zones = original.map(zone => Object.assign({}, zone, {suggested: zone.code === "Asia/Jerusalem"}))
            compare(picker.filteredZones.length, original.length)
            compare(picker.filteredZones.map(zone => zone.code).join(","), original.map(zone => zone.code).join(","))
            picker.searchText = "London"
            compare(picker.filteredZones.length, 1)
            compare(picker.filteredZones[0].code, "Europe/London")
            picker.selectedCode = "Europe/London"
            picker.searchText = ""
            compare(picker.filteredZones.length, original.length)
            compare(picker.filteredZones[0].code, "Asia/Jerusalem")
            compare(picker.filteredZones.filter(zone => zone.code === "Europe/London").length, 1)
            compare(picker.selectedCode, "Europe/London")
            picker.zones = original
        }
        function test_scrollbar_gutter_and_visibility_data() {
            const rows = []
            for (const rtl of [false, true]) for (const scale of [1, 0.55])
                rows.push({tag: (rtl ? "RTL" : "LTR") + "-" + scale, rtl, scale})
            return rows
        }
        function test_scrollbar_gutter_and_visibility(data) {
            const original = picker.zones
            picker.rtl = data.rtl
            picker.uiScale = data.scale
            const view = picker.resultView
            const bar = view.verticalScrollBar
            const frame = findChild(picker, "timezone-results-frame")
            compare(bar.parent, frame)
            compare(bar.policy, ScrollBar.AsNeeded)
            tryCompare(bar, "visible", false) // Four rows already fit.
            picker.searchText = "zzzz"
            tryCompare(view, "count", 0)
            tryCompare(bar, "visible", false)
            picker.searchText = "London"
            tryCompare(view, "count", 1)
            tryCompare(bar, "visible", false)
            picker.searchText = ""
            picker.zones = Array.from({length: 30}, (_, index) => ({
                code: "zone-" + index, label: "Timezone " + index, english: "Timezone " + index,
                suggested: index === 0
            }))
            tryCompare(view, "count", 30)
            tryCompare(bar, "visible", true)
            view.positionViewAtBeginning()
            view.forceLayout()
            const row = view.itemAtIndex(0)
            verify(row)
            const rowPosition = row.mapToItem(frame, 0, 0)
            if (data.rtl) {
                compare(bar.x, 0)
                verify(rowPosition.x >= bar.x + bar.width + Math.round(8 * data.scale) - 1)
                verify(Math.abs(rowPosition.x + row.width - frame.width) <= 1)
            } else {
                compare(bar.x + bar.width, frame.width)
                verify(rowPosition.x + row.width <= bar.x - Math.round(8 * data.scale) + 1)
                verify(Math.abs(rowPosition.x) <= 1)
            }
            verify(bar.x >= 0 && bar.x + bar.width <= frame.width + 1)
            const initialX = view.contentX
            mouseDrag(bar, bar.width / 2, bar.height * 0.15, 0, bar.height * 0.5)
            tryVerify(() => view.contentY > 0)
            compare(view.contentX, initialX)
            picker.zones = original
        }
        function test_empty_search_restores_all_rows_after_search_or_manual_selection() {
            const original = picker.zones
            picker.zones = original.map(zone => Object.assign({}, zone, {suggested: zone.code === "Asia/Jerusalem"}))
            for (const query of ["", "   ", "London", "zzzz", ""]) {
                picker.searchText = query
                if (query.trim().length === 0) compare(picker.filteredZones.length, original.length)
            }
            picker.selectedCode = "Europe/Paris"
            picker.searchText = ""
            compare(picker.filteredZones[0].code, "Asia/Jerusalem")
            compare(picker.filteredZones[3].code, "Europe/Paris")
            compare(picker.filteredZones.length, original.length)
            picker.zones = original
        }
        function test_touch_and_wheel_scroll_results_without_horizontal_drift() {
            const original = picker.zones
            picker.zones = original.concat(original.map(zone => Object.assign({}, zone, {code: zone.code + "-duplicate"})))
            picker.searchText = ""
            wait(100)
            const view = picker.resultView
            verify(view.interactive)
            mouseWheel(view, view.width / 2, view.height / 2, 0, -240)
            tryVerify(() => view.contentY > 0)
            mouseWheel(view, view.width / 2, view.height / 2, -120, 0)
            wait(400)
            compare(view.contentX, 0)
            view.cancelFlick()
            view.positionViewAtBeginning()
            wait(100)
            const touch = touchEvent(view)
            touch.press(0, view, view.width / 2, view.height - 10).commit()
            wait(20)
            for (let y = view.height - 30; y >= 5; y -= 15) {
                touch.move(0, view, view.width / 2, y).commit()
                wait(20)
            }
            touch.release(0, view, view.width / 2, 5).commit()
            tryVerify(() => view.contentY > 0)
            picker.zones = original
        }
        function test_five_entries_fit_below_map() {
            const original = picker.zones
            picker.zones = original.concat(original.map(zone => Object.assign({}, zone, {code: zone.code + "-duplicate"})))
            picker.resultView.positionViewAtBeginning()
            wait(100)
            const row = picker.resultView.itemAtIndex(4)
            verify(row)
            verify(row.y + row.height - picker.resultView.contentY <= picker.resultView.height + 1)
            picker.zones = original
        }
    }
}
