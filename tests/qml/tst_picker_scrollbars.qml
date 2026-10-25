import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml"

Item {
    id: host
    width: 1000
    height: 900
    readonly property var regions: [
        {code:"en",label:"English (United States)",english:"United States"},
        {code:"en_AU",label:"English (Australia)",english:"Australia"},
        {code:"en_CA",label:"English (Canada)",english:"Canada"},
        {code:"en_IE",label:"English (Ireland)",english:"Ireland"},
        {code:"en_NZ",label:"English (New Zealand)",english:"New Zealand"},
        {code:"en_ZA",label:"English (South Africa)",english:"South Africa"},
        {code:"en_GB",label:"English (United Kingdom)",english:"United Kingdom"}
    ]
    Component {
        id: pickerComponent
        LanguagePicker {
            selectedCode: "en"
            onLanguageSelected: code => selectedCode = code
        }
    }
    TestCase {
        name: "PickerScrollbars"
        when: windowShown
        function init() { failOnWarning(/.?/) }
        function rows(grouped) {
            return host.regions.map((row,index) => Object.assign({},row,
                {group:grouped ? index < 2 ? "Suggested" : "Other" : ""}))
        }
        function settled(picker) {
            picker.cancelFlick()
            picker.forceLayout()
            wait(80)
        }
        function test_short_long_empty_and_resize_data() {
            const data = []
            for (const rtl of [false,true]) for (const scale of [1,0.65])
                for (const grouped of [false,true])
                    data.push({tag:(rtl?"RTL":"LTR")+"-"+scale+"-"+(grouped?"grouped":"plain"),rtl,scale,grouped})
            return data
        }
        function test_short_long_empty_and_resize(data) {
            const shortRows = rows(data.grouped)
            const picker = createTemporaryObject(pickerComponent,host,{
                width:520,height:500,languages:shortRows,rtl:data.rtl,
                uiScale:data.scale,grouped:data.grouped
            })
            verify(picker)
            settled(picker)
            const bar = picker.verticalScrollBar
            verify(picker.contentHeight < picker.height)
            tryCompare(bar,"visible",false,1000,"A list that fits must not show a scrollbar")
            const rowWidth = picker.itemAtIndex(0).width
            const left = picker.leftMargin, right = picker.rightMargin
            picker.forceActiveFocus()
            keyClick(Qt.Key_End)
            compare(picker.selectedCode,"en_GB")
            settled(picker)
            const top = picker.contentY
            mouseWheel(picker,200,160,0,-120)
            wait(350)
            verify(Math.abs(picker.contentY-top) < 1,"A short list scrolled despite fitting")

            picker.languages = shortRows.concat(Array.from({length:40},(_,index) => ({
                code:"extra-"+index,label:"Extra region "+index,english:"Extra region "+index,
                group:data.grouped?"Other":""
            })))
            settled(picker)
            tryCompare(bar,"visible",true)
            wait(800)
            verify(bar.contentItem.opacity > 0,"An overflowing list's thumb faded away")
            compare(picker.leftMargin,left)
            compare(picker.rightMargin,right)
            compare(picker.itemAtIndex(0).width,rowWidth)
            verify(data.rtl ? bar.x === 0 : Math.abs(bar.x+bar.width-picker.width) < 1)
            picker.positionViewAtBeginning()
            const before = picker.contentY
            mouseWheel(picker,200,160,0,-240)
            tryVerify(() => picker.contentY > before+10)
            wait(350)
            picker.languages = shortRows
            settled(picker)
            tryCompare(bar,"visible",false)
            verify(picker.atYBeginning)
            compare(picker.itemAtIndex(0).width,rowWidth)

            // Use the measured content, including group headers and row wrap,
            // rather than a hardcoded number-of-rows visibility heuristic.
            picker.height = picker.contentHeight
            settled(picker)
            tryCompare(bar,"visible",false)
            picker.height = picker.contentHeight-2
            settled(picker)
            tryCompare(bar,"visible",true)
            picker.height = 500
            settled(picker)
            tryCompare(bar,"visible",false)
            picker.languages = []
            settled(picker)
            compare(picker.count,0)
            tryCompare(bar,"visible",false)
            picker.languages = shortRows.slice(0,1)
            settled(picker)
            compare(picker.count,1)
            tryCompare(bar,"visible",false)
        }
        function test_wrapped_rows_follow_actual_content_height_data() {
            return [{tag:"LTR",rtl:false},{tag:"RTL",rtl:true}]
        }
        function test_wrapped_rows_follow_actual_content_height(data) {
            const picker = createTemporaryObject(pickerComponent,host,{
                width:220,height:500,rtl:data.rtl,languages:host.regions.slice(0,3)
                    .map(row => Object.assign({},row,{label:(row.label+" ").repeat(4)}))
            })
            verify(picker)
            settled(picker)
            verify(picker.itemAtIndex(0).height > 44)
            picker.height = picker.contentHeight+10
            settled(picker)
            tryCompare(picker.verticalScrollBar,"visible",false)
            picker.height = picker.contentHeight-10
            settled(picker)
            tryCompare(picker.verticalScrollBar,"visible",true)
        }
    }
}
