Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS - Monetary Unit Sampling (PPS) for audit evidence
'  Pure VBA, ASCII only (no locale/encoding issues).
'  Conforms to ISA 530 sampling + evaluation workflow.
' ------------------------------------------------------------
'  Macros:
'   MUS_Sample   - draw the sample, write a self-documenting
'                  working paper sheet, and append an audit log row.
'   MUS_Evaluate - (run on a result sheet after entering Audit_Value)
'                  the sheet already auto-computes UEL/verdict via
'                  live formulas; this just jumps you to the summary.
'
'  Method:
'   R   = -ln(1 - confidence)            ' reliability factor, 0 errors, Poisson
'   interval = PM / R                    ' sampling interval
'   amount >= interval  -> Key Item (100% examined)
'   amount <  interval  -> systematic PPS selection from a seeded start
'   Basic precision      = interval * R
'   Tainting             = (Book - Audit) / Book
'   Projected misstmt    = tainting * interval   (PPS)
'                        = Book - Audit          (Key Item, examined 100%)
'   Upper error limit    = basic precision + SUM(projected)
'   Verdict: UEL <= PM -> Acceptable, else Further audit work needed
'
'  Deterministic: same (range, PM, confidence, seed) = same sample.
'  Every run is recorded on the "MUS_Log" sheet (audit trail).
' ============================================================

Private Const LOG_SHEET As String = "MUS_Log"

Public Sub MUS_Sample()
    Dim rng As Range
    On Error Resume Next
    Set rng = Application.InputBox( _
        "Select the data range (top row = header).", _
        "MUS - Range", Selection.Address, Type:=8)
    On Error GoTo 0
    If rng Is Nothing Then Exit Sub

    Dim srcWs As Worksheet: Set srcWs = rng.Worksheet
    Dim data As Variant: data = rng.Value
    If Not IsArray(data) Then MsgBox "Select multiple rows.", vbExclamation: Exit Sub
    Dim nRows As Long, nCols As Long
    nRows = UBound(data, 1): nCols = UBound(data, 2)
    If nRows < 2 Then MsgBox "Need header + data (>=2 rows).", vbExclamation: Exit Sub

    ' --- amount column ---
    Dim headers As String, c As Long
    For c = 1 To nCols
        headers = headers & c & ".  " & data(1, c) & vbCrLf
    Next c
    Dim s As String
    s = InputBox("Amount column number:" & vbCrLf & vbCrLf & headers, "MUS - Amount column")
    If Not IsNumeric(s) Then Exit Sub
    Dim amtCol As Long: amtCol = CLng(s)
    If amtCol < 1 Or amtCol > nCols Then MsgBox "Enter 1 to " & nCols & ".", vbExclamation: Exit Sub

    ' --- parameters ---
    s = InputBox("Performance materiality (PM / tolerable misstatement):", "MUS - PM", "1000000")
    If Not IsNumeric(s) Then Exit Sub
    Dim pm As Double: pm = CDbl(s)
    If pm <= 0 Then MsgBox "PM must be positive.", vbExclamation: Exit Sub

    s = InputBox("Confidence (0.90 / 0.95 / 0.99):", "MUS - Confidence", "0.95")
    If Not IsNumeric(s) Then Exit Sub
    Dim conf As Double: conf = CDbl(s)
    If conf < 0.5 Then conf = 0.5
    If conf > 0.999 Then conf = 0.999

    s = InputBox("Seed (same seed = same sample, for reproducibility):", "MUS - Seed", "42")
    If Not IsNumeric(s) Then Exit Sub
    Dim seed As Double: seed = CDbl(s)

    Dim rf As Double: rf = -Log(1# - conf)          ' VBA Log = natural log
    Dim interval As Double: interval = pm / rf

    ' --- population: positive items only; split Key Item / PPS ---
    Dim keyIdx() As Long, ppsIdx() As Long, ppsAmt() As Double
    Dim nKey As Long, nPps As Long, i As Long
    Dim popN As Long, popTotal As Double
    ReDim keyIdx(1 To nRows): ReDim ppsIdx(1 To nRows): ReDim ppsAmt(1 To nRows)
    For i = 2 To nRows
        If IsNumeric(data(i, amtCol)) Then
            Dim a As Double: a = CDbl(data(i, amtCol))
            If a > 0 Then
                popN = popN + 1: popTotal = popTotal + a
                If a >= interval Then
                    nKey = nKey + 1: keyIdx(nKey) = i
                Else
                    nPps = nPps + 1: ppsIdx(nPps) = i: ppsAmt(nPps) = a
                End If
            End If
        End If
    Next i

    ' --- selection: Key Items (all) then systematic PPS ---
    Dim selRow() As Long, selType() As String, nSel As Long
    ReDim selRow(1 To nRows): ReDim selType(1 To nRows)
    For i = 1 To nKey
        nSel = nSel + 1: selRow(nSel) = keyIdx(i): selType(nSel) = "Key Item"
    Next i

    Dim startPt As Double: startPt = 0
    If nPps > 0 Then
        Dim cum() As Double, total As Double: ReDim cum(1 To nPps)
        For i = 1 To nPps
            total = total + ppsAmt(i): cum(i) = total
        Next i
        Rnd -1: Randomize seed
        startPt = 0.0001 + Rnd() * (interval - 0.0001)
        Dim cnt As Long, j As Long, last As Long, idx As Long
        cnt = Int((total - startPt) / interval) + 1
        If cnt < 1 Then cnt = 1
        last = -1
        For j = 0 To cnt - 1
            Dim point As Double: point = startPt + j * interval
            idx = 0
            For i = 1 To nPps
                If cum(i) >= point Then idx = i: Exit For
            Next i
            If idx >= 1 And idx <> last Then
                nSel = nSel + 1: selRow(nSel) = ppsIdx(idx): selType(nSel) = "PPS Sample"
                last = idx
            End If
        Next j
    End If

    If nSel = 0 Then MsgBox "No sample selected. Check PM / amount column.", vbExclamation: Exit Sub

    Dim basicPrec As Double: basicPrec = interval * rf

    ' --- result sheet (self-documenting working paper) ---
    Dim ws As Worksheet: Set ws = ActiveWorkbook.Worksheets.Add
    Dim runId As Long: runId = NextRunId()
    On Error Resume Next
    ws.Name = "MUS run" & runId
    On Error GoTo 0

    ' parameter block (rows 1..22, col A label / col B value)
    Dim pc As Long: pc = nCols + 5                 ' Projected_MS column index
    Dim HR As Long: HR = 24                         ' table header row
    Dim firstDataRow As Long: firstDataRow = HR + 1
    Dim lastDataRow As Long: lastDataRow = HR + nSel

    ws.Range("A1").Value = "axon MUS - Working Paper"
    PutP ws, 2, "Run ID", runId
    PutP ws, 3, "Run time", Format(Now, "yyyy-mm-dd hh:nn:ss")
    PutP ws, 4, "Performed by", Environ$("USERNAME")
    PutP ws, 5, "Workbook", ActiveWorkbook.Name
    PutP ws, 6, "Source sheet", srcWs.Name
    PutP ws, 7, "Source range", rng.Address(False, False)
    PutP ws, 8, "Amount column", CStr(data(1, amtCol))
    PutP ws, 9, "Population count (positive)", popN
    PutP ws, 10, "Population total", popTotal
    PutP ws, 11, "Performance materiality (PM)", pm
    PutP ws, 12, "Confidence", conf
    PutP ws, 13, "Reliability factor (R)", rf
    PutP ws, 14, "Sampling interval", interval
    PutP ws, 15, "Seed", seed
    PutP ws, 16, "Random start", startPt
    PutP ws, 17, "Key items", nKey
    PutP ws, 18, "PPS samples", nSel - nKey
    PutP ws, 19, "Sample size", nSel
    PutP ws, 20, "Basic precision", basicPrec
    ' Upper error limit + verdict are LIVE formulas (update as Audit_Value is edited)
    ws.Range("A21").Value = "Upper error limit (UEL)"
    ws.Range("B21").FormulaR1C1 = "=R20C2+SUM(R" & firstDataRow & "C" & pc & ":R" & lastDataRow & "C" & pc & ")"
    ws.Range("A22").Value = "Verdict"
    ws.Range("B22").FormulaR1C1 = "=IF(R21C2<=R11C2,""Acceptable"",""Further audit work needed"")"

    ' table header
    For c = 1 To nCols
        ws.Cells(HR, c).Value = data(1, c)
    Next c
    ws.Cells(HR, nCols + 1).Value = "Type"
    ws.Cells(HR, nCols + 2).Value = "Book_Value"
    ws.Cells(HR, nCols + 3).Value = "Audit_Value"
    ws.Cells(HR, nCols + 4).Value = "Tainting"
    ws.Cells(HR, nCols + 5).Value = "Projected_MS"

    ' table rows + live evaluation formulas
    Dim r As Long, srcRow As Long, rowAbs As Long
    For r = 1 To nSel
        srcRow = selRow(r)
        rowAbs = HR + r
        For c = 1 To nCols
            ws.Cells(rowAbs, c).Value = data(srcRow, c)
        Next c
        ws.Cells(rowAbs, nCols + 1).Value = selType(r)
        ws.Cells(rowAbs, nCols + 2).Value = data(srcRow, amtCol)   ' Book_Value
        ws.Cells(rowAbs, nCols + 3).Value = data(srcRow, amtCol)   ' Audit_Value (auditor edits)
        ' Tainting = (Book - Audit) / Book  (0 if Book<=0)
        ws.Cells(rowAbs, nCols + 4).FormulaR1C1 = "=IF(RC[-2]>0,(RC[-2]-RC[-1])/RC[-2],0)"
        ' Projected: Key Item = Book - Audit ; PPS = tainting * interval (B14)
        If selType(r) = "Key Item" Then
            ws.Cells(rowAbs, nCols + 5).FormulaR1C1 = "=RC[-3]-RC[-2]"
        Else
            ws.Cells(rowAbs, nCols + 5).FormulaR1C1 = "=RC[-1]*R14C2"
        End If
    Next r

    ws.Columns.AutoFit
    ws.Range("A1").Font.Bold = True
    ws.Range("A2:A22").Font.Bold = False
    ws.Rows(HR).Font.Bold = True

    ' --- append audit log row ---
    WriteLog runId, srcWs.Name, rng.Address(False, False), CStr(data(1, amtCol)), _
             popN, popTotal, pm, conf, rf, interval, seed, startPt, _
             nKey, nSel - nKey, nSel, basicPrec, ws.Name

    Application.Goto ws.Range("A1"), True
    MsgBox "Sample: " & nSel & " (Key " & nKey & " / PPS " & (nSel - nKey) & ")" & vbCrLf & _
           "Sheet: " & ws.Name & "   Logged as run " & runId & " on '" & LOG_SHEET & "'." & vbCrLf & vbCrLf & _
           "Next: fill the Audit_Value column. UEL and Verdict update automatically.", _
           vbInformation, "axon MUS"
End Sub

' Jump to the summary block of the active result sheet.
Public Sub MUS_Evaluate()
    On Error Resume Next
    Application.Goto ActiveSheet.Range("A21"), True
    On Error GoTo 0
    MsgBox "UEL and Verdict (cells B21/B22) recompute live from the Audit_Value column." & vbCrLf & _
           "If UEL <= PM the population is acceptable; otherwise extend procedures.", _
           vbInformation, "axon MUS - Evaluate"
End Sub

' --- helpers ---

Private Sub PutP(ws As Worksheet, rowN As Long, label As String, v As Variant)
    ws.Cells(rowN, 1).Value = label
    ws.Cells(rowN, 2).Value = v
End Sub

Private Function NextRunId() As Long
    Dim lg As Worksheet: Set lg = LogSheet()
    Dim lastR As Long: lastR = lg.Cells(lg.Rows.Count, 1).End(xlUp).Row
    If lastR < 2 Then
        NextRunId = 1
    Else
        NextRunId = lg.Cells(lastR, 1).Value + 1
    End If
End Function

Private Function LogSheet() As Worksheet
    Dim lg As Worksheet
    On Error Resume Next
    Set lg = ActiveWorkbook.Worksheets(LOG_SHEET)
    On Error GoTo 0
    If lg Is Nothing Then
        Set lg = ActiveWorkbook.Worksheets.Add
        lg.Name = LOG_SHEET
        Dim hdr As Variant, k As Long
        hdr = Array("Run_ID", "Timestamp", "User", "Workbook", "Source_Sheet", _
            "Source_Range", "Amount_Col", "Pop_Count", "Pop_Total", "PM", _
            "Confidence", "R_Factor", "Interval", "Seed", "Random_Start", _
            "Key_Items", "PPS_Samples", "Sample_Size", "Basic_Precision", "Result_Sheet")
        For k = 0 To UBound(hdr)
            lg.Cells(1, k + 1).Value = hdr(k)
        Next k
        lg.Rows(1).Font.Bold = True
    End If
    Set LogSheet = lg
End Function

Private Sub WriteLog(runId As Long, srcSheet As String, srcRange As String, amtName As String, _
        popN As Long, popTotal As Double, pm As Double, conf As Double, rf As Double, _
        interval As Double, seed As Double, startPt As Double, nKey As Long, nPps As Long, _
        nSel As Long, basicPrec As Double, resultSheet As String)
    Dim lg As Worksheet: Set lg = LogSheet()
    Dim r As Long: r = lg.Cells(lg.Rows.Count, 1).End(xlUp).Row + 1
    lg.Cells(r, 1).Value = runId
    lg.Cells(r, 2).Value = Format(Now, "yyyy-mm-dd hh:nn:ss")
    lg.Cells(r, 3).Value = Environ$("USERNAME")
    lg.Cells(r, 4).Value = ActiveWorkbook.Name
    lg.Cells(r, 5).Value = srcSheet
    lg.Cells(r, 6).Value = srcRange
    lg.Cells(r, 7).Value = amtName
    lg.Cells(r, 8).Value = popN
    lg.Cells(r, 9).Value = popTotal
    lg.Cells(r, 10).Value = pm
    lg.Cells(r, 11).Value = conf
    lg.Cells(r, 12).Value = rf
    lg.Cells(r, 13).Value = interval
    lg.Cells(r, 14).Value = seed
    lg.Cells(r, 15).Value = startPt
    lg.Cells(r, 16).Value = nKey
    lg.Cells(r, 17).Value = nPps
    lg.Cells(r, 18).Value = nSel
    lg.Cells(r, 19).Value = basicPrec
    lg.Cells(r, 20).Value = resultSheet
End Sub

' ============================================================
'  Add-in (.xlam) ribbon button - shows under the "Add-ins" tab
'  when this file is loaded as an Excel Add-in. Pure VBA, no XML.
' ============================================================
Public Sub Auto_Open()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
    Dim cb As CommandBar
    Set cb = Application.CommandBars.Add(Name:="axon MUS", Position:=msoBarTop, Temporary:=True)
    Dim b As CommandBarButton
    Set b = cb.Controls.Add(Type:=msoControlButton)
    b.Caption = "MUS Sample"
    b.Style = msoButtonCaption
    b.OnAction = "MUS_Sample"
    cb.Visible = True
End Sub

Public Sub Auto_Close()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
End Sub
