Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS - Monetary Unit Sampling (PPS) for audit evidence
'  Pure VBA, ASCII only. ISA 530 / AICPA MUS sampling + evaluation.
' ------------------------------------------------------------
'  MUS_Sample   - draw the sample, write the table, attach a summary
'                 NOTE to cell A1, and append an audit-trail log row.
'  MUS_Evaluate - run AFTER entering Audit_Value; recomputes the
'                 precise upper error limit (UEL) with ranked
'                 incremental reliability factors and refreshes the note.
'  R(k) = Poisson reliability factor (pure VBA). interval = PM / R0.
' ============================================================

Private Const LOG_SHEET As String = "MUS_Log"
Private Const T_KEY As String = "Key Item"
Private Const T_PPS As String = "PPS Sample"
Private Const H_TYPE As String = "Type"
Private Const H_BOOK As String = "Book_Value"
Private Const H_AUDIT As String = "Audit_Value"
Private Const H_TAINT As String = "Tainting"
Private Const H_PROJ As String = "Projected_MS"
Private Const MARKER As String = "===== Evaluation ====="

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

    Dim headers As String, c As Long
    For c = 1 To nCols
        headers = headers & c & ".  " & data(1, c) & vbCrLf
    Next c
    Dim s As String
    s = InputBox("Amount column number:" & vbCrLf & vbCrLf & headers, "MUS - Amount column")
    If Not IsNumeric(s) Then Exit Sub
    Dim amtCol As Long: amtCol = CLng(s)
    If amtCol < 1 Or amtCol > nCols Then MsgBox "Enter 1 to " & nCols & ".", vbExclamation: Exit Sub

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

    Dim itemName As String
    itemName = Trim(InputBox("Sampling item name (used for the sheet name):", "MUS - Item name", "Sample"))
    If itemName = "" Then itemName = "Sample"

    Dim rf As Double: rf = ReliabilityFactor(conf, 0)
    Dim interval As Double: interval = pm / rf

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

    Dim selRow() As Long, selType() As String, nSel As Long
    ReDim selRow(1 To nRows): ReDim selType(1 To nRows)
    For i = 1 To nKey
        nSel = nSel + 1: selRow(nSel) = keyIdx(i): selType(nSel) = T_KEY
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
                nSel = nSel + 1: selRow(nSel) = ppsIdx(idx): selType(nSel) = T_PPS
                last = idx
            End If
        Next j
    End If

    If nSel = 0 Then MsgBox "No sample selected. Check PM / amount column.", vbExclamation: Exit Sub
    Dim basicPrec As Double: basicPrec = interval * rf

    Dim ws As Worksheet: Set ws = ActiveWorkbook.Worksheets.Add
    Dim runId As Long: runId = NextRunId()
    NameSheet ws, itemName, runId
    ws.Tab.Color = RGB(46, 117, 182)               ' distinct tab color

    ' sheet-scoped names used by formulas and re-evaluation
    AddNum ws, "MUS_PM", pm
    AddNum ws, "MUS_CONF", conf
    AddNum ws, "MUS_INTERVAL", interval

    ' table: header row 1, data from row 2
    For c = 1 To nCols
        ws.Cells(1, c).Value = data(1, c)
    Next c
    ws.Cells(1, nCols + 1).Value = H_TYPE
    ws.Cells(1, nCols + 2).Value = H_BOOK
    ws.Cells(1, nCols + 3).Value = H_AUDIT
    ws.Cells(1, nCols + 4).Value = H_TAINT
    ws.Cells(1, nCols + 5).Value = H_PROJ

    Dim r As Long, srcRow As Long, rowAbs As Long
    For r = 1 To nSel
        srcRow = selRow(r)
        rowAbs = 1 + r
        For c = 1 To nCols
            ws.Cells(rowAbs, c).Value = data(srcRow, c)
        Next c
        ws.Cells(rowAbs, nCols + 1).Value = selType(r)
        ws.Cells(rowAbs, nCols + 2).Value = data(srcRow, amtCol)
        ws.Cells(rowAbs, nCols + 3).Value = data(srcRow, amtCol)
        ws.Cells(rowAbs, nCols + 4).FormulaR1C1 = "=IF(RC[-2]>0,(RC[-2]-RC[-1])/RC[-2],0)"
        If selType(r) = T_KEY Then
            ws.Cells(rowAbs, nCols + 5).FormulaR1C1 = "=RC[-3]-RC[-2]"
        Else
            ws.Cells(rowAbs, nCols + 5).FormulaR1C1 = "=RC[-1]*MUS_INTERVAL"
        End If
    Next r

    ' number formats
    ws.Range(ws.Cells(2, nCols + 2), ws.Cells(1 + nSel, nCols + 3)).NumberFormat = "#,##0"
    ws.Range(ws.Cells(2, nCols + 4), ws.Cells(1 + nSel, nCols + 4)).NumberFormat = "0.0%"
    ws.Range(ws.Cells(2, nCols + 5), ws.Cells(1 + nSel, nCols + 5)).NumberFormat = "#,##0"
    ws.Rows(1).Font.Bold = True
    ws.Columns.AutoFit

    ' static part of the summary note
    Dim st As String
    st = "MUS Sampling Summary" & vbLf & String(34, "-") & vbLf
    st = st & "Item: " & itemName & vbLf
    st = st & "Run ID: " & runId & "    Run time: " & Format(Now, "yyyy-mm-dd hh:nn:ss") & vbLf
    st = st & "Performed by: " & Environ$("USERNAME") & vbLf
    st = st & "Workbook: " & ActiveWorkbook.Name & vbLf
    st = st & "Source: " & srcWs.Name & "!" & rng.Address(False, False) & "   Amount: " & CStr(data(1, amtCol)) & vbLf & vbLf
    st = st & "[Planning]" & vbLf
    st = st & "Population: " & Fmt(popN) & " items / " & Fmt(popTotal) & vbLf
    st = st & "PM (tolerable): " & Fmt(pm) & vbLf
    st = st & "Confidence: " & Format(conf * 100, "0") & "%" & vbLf
    st = st & "Reliability factor R0: " & Fmt(rf) & vbLf
    st = st & "Sampling interval: " & Fmt(interval) & vbLf
    st = st & "Seed: " & Fmt(seed) & "   Random start: " & Fmt(startPt) & vbLf
    st = st & "Sample: " & Fmt(nSel) & "  (Key " & Fmt(nKey) & " / PPS " & Fmt(nSel - nKey) & ")" & vbLf
    st = st & "Basic precision: " & Fmt(basicPrec) & vbLf & vbLf

    SetNote ws, st & MARKER & vbLf & "(pending - run MUS_Evaluate after entering Audit_Value)"
    EvaluateSheet ws

    WriteLog runId, itemName, srcWs.Name, rng.Address(False, False), CStr(data(1, amtCol)), _
             popN, popTotal, pm, conf, rf, interval, seed, startPt, _
             nKey, nSel - nKey, nSel, basicPrec, ws.Name

    Application.Goto ws.Range("A1"), True
    MsgBox "Sample: " & nSel & " (Key " & nKey & " / PPS " & (nSel - nKey) & ")" & vbCrLf & _
           "Sheet: " & ws.Name & "   (summary is the note on cell A1)" & vbCrLf & _
           "Logged as run " & runId & " on '" & LOG_SHEET & "'." & vbCrLf & vbCrLf & _
           "Next: fill Audit_Value, then run MUS_Evaluate for the precise UEL.", _
           vbInformation, "axon MUS"
End Sub

Public Sub MUS_Evaluate()
    On Error GoTo fail
    EvaluateSheet ActiveSheet
    MsgBox "Evaluation refreshed in the note on cell A1." & vbCrLf & _
           "UEL uses ranked incremental reliability factors." & vbCrLf & _
           "If UEL <= PM the population is acceptable; otherwise extend procedures.", _
           vbInformation, "axon MUS - Evaluate"
    Exit Sub
fail:
    MsgBox "Run this on a MUS result sheet (one made by MUS_Sample).", vbExclamation
End Sub

' --- core evaluation: ranked incremental UEL, refreshes the A1 note ---
Private Sub EvaluateSheet(ws As Worksheet)
    Dim pm As Double: pm = ws.Evaluate("MUS_PM")
    Dim conf As Double: conf = ws.Evaluate("MUS_CONF")
    Dim interval As Double: interval = ws.Evaluate("MUS_INTERVAL")
    Dim r0 As Double: r0 = ReliabilityFactor(conf, 0)

    Dim found As Range
    Set found = ws.Cells.Find(What:=H_BOOK, LookAt:=xlWhole, MatchCase:=True)
    If found Is Nothing Then Err.Raise 5
    Dim HR As Long, bookCol As Long, typeCol As Long, auditCol As Long
    HR = found.Row: bookCol = found.Column
    typeCol = bookCol - 1: auditCol = bookCol + 1

    Dim proj() As Double: ReDim proj(1 To 1)
    Dim m As Long: m = 0
    Dim keyActual As Double
    Dim r As Long, book As Double, audit As Double, taint As Double
    r = HR + 1
    Do While Len(CStr(ws.Cells(r, typeCol).Value)) > 0
        book = SafeNum(ws.Cells(r, bookCol).Value)
        audit = SafeNum(ws.Cells(r, auditCol).Value)
        If ws.Cells(r, typeCol).Value = T_KEY Then
            If book - audit > 0 Then keyActual = keyActual + (book - audit)
        ElseIf book > 0 Then
            taint = (book - audit) / book
            If taint > 0 Then
                m = m + 1: ReDim Preserve proj(1 To m): proj(m) = taint * interval
            End If
        End If
        r = r + 1
    Loop

    Dim i As Long, j As Long, tmp As Double
    For i = 1 To m - 1
        For j = 1 To m - i
            If proj(j) < proj(j + 1) Then tmp = proj(j): proj(j) = proj(j + 1): proj(j + 1) = tmp
        Next j
    Next i

    Dim basicPrec As Double: basicPrec = r0 * interval
    Dim rankedSum As Double, projPPS As Double, rPrev As Double, rCur As Double
    rPrev = r0
    For j = 1 To m
        rCur = ReliabilityFactor(conf, j)
        rankedSum = rankedSum + proj(j) * (rCur - rPrev)
        projPPS = projPPS + proj(j)
        rPrev = rCur
    Next j

    Dim uel As Double: uel = basicPrec + rankedSum + keyActual
    Dim projectedTotal As Double: projectedTotal = projPPS + keyActual
    Dim incrementalAllow As Double: incrementalAllow = rankedSum - projPPS
    Dim verdict As String
    If uel <= pm Then verdict = "Acceptable (UEL <= PM)" Else verdict = "Further audit work needed (UEL > PM)"

    Dim ev As String
    ev = MARKER & vbLf
    ev = ev & "Projected misstatement: " & Fmt(projectedTotal) & vbLf
    ev = ev & "Incremental allowance: " & Fmt(incrementalAllow) & vbLf
    ev = ev & "Upper error limit (UEL): " & Fmt(uel) & vbLf
    ev = ev & "Verdict: " & verdict

    Dim full As String, pos As Long
    full = GetNote(ws)
    pos = InStr(full, MARKER)
    If pos > 0 Then full = Left$(full, pos - 1)
    SetNote ws, full & ev
End Sub

' --- reliability factor R(k): Poisson inversion, no worksheet dependency ---
Private Function ReliabilityFactor(ByVal conf As Double, ByVal k As Long) As Double
    Dim target As Double: target = 1# - conf
    Dim lo As Double, hi As Double, mid As Double, it As Long
    lo = 0#: hi = 50# + 2# * k
    For it = 1 To 200
        mid = (lo + hi) / 2#
        If PoissonCDF(k, mid) > target Then lo = mid Else hi = mid
    Next it
    ReliabilityFactor = (lo + hi) / 2#
End Function

Private Function PoissonCDF(ByVal k As Long, ByVal x As Double) As Double
    Dim term As Double, sum As Double, i As Long
    term = Exp(-x): sum = term
    For i = 1 To k
        term = term * x / i: sum = sum + term
    Next i
    PoissonCDF = sum
End Function

' --- formatting / helpers ---
Private Function Fmt(v As Variant) As String
    If IsNumeric(v) Then Fmt = Format(v, "#,##0.####") Else Fmt = CStr(v)
End Function

Private Function SafeNum(v As Variant) As Double
    If IsNumeric(v) Then SafeNum = CDbl(v) Else SafeNum = 0#
End Function

Private Sub AddNum(ws As Worksheet, nm As String, v As Double)
    ws.Names.Add Name:=nm, RefersTo:="=" & Replace(Trim(Str(v)), " ", "")
End Sub

Private Sub NameSheet(ws As Worksheet, itemName As String, runId As Long)
    Dim base As String: base = "MUS_" & CleanName(itemName)
    Dim nm As String: nm = Left$(base, 31)
    On Error Resume Next
    ws.Name = nm
    If ws.Name <> nm Then ws.Name = Left$(base, 27) & "_" & runId
    If ws.Name <> Left$(base, 27) & "_" & runId And ws.Name <> nm Then ws.Name = "MUS_" & runId
    On Error GoTo 0
End Sub

Private Function CleanName(ByVal t As String) As String
    Dim bad As Variant, x As Variant
    bad = Array(":", "\", "/", "?", "*", "[", "]")
    For Each x In bad
        t = Replace(t, CStr(x), "")
    Next x
    CleanName = Trim(t)
End Function

Private Sub SetNote(ws As Worksheet, text As String)
    Dim cell As Range: Set cell = ws.Range("A1")
    If cell.Comment Is Nothing Then cell.AddComment
    cell.Comment.Text Text:=text
    cell.Comment.Shape.TextFrame.AutoSize = True
End Sub

Private Function GetNote(ws As Worksheet) As String
    If ws.Range("A1").Comment Is Nothing Then GetNote = "" Else GetNote = ws.Range("A1").Comment.Text
End Function

Private Function NextRunId() As Long
    Dim lg As Worksheet: Set lg = LogSheet()
    Dim lastR As Long: lastR = lg.Cells(lg.Rows.Count, 1).End(xlUp).Row
    If lastR < 2 Then NextRunId = 1 Else NextRunId = lg.Cells(lastR, 1).Value + 1
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
        hdr = Array("Run_ID", "Item", "Timestamp", "User", "Workbook", "Source_Sheet", _
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

Private Sub WriteLog(runId As Long, itemName As String, srcSheet As String, srcRange As String, _
        amtName As String, popN As Long, popTotal As Double, pm As Double, conf As Double, _
        rf As Double, interval As Double, seed As Double, startPt As Double, nKey As Long, _
        nPps As Long, nSel As Long, basicPrec As Double, resultSheet As String)
    Dim lg As Worksheet: Set lg = LogSheet()
    Dim r As Long: r = lg.Cells(lg.Rows.Count, 1).End(xlUp).Row + 1
    lg.Cells(r, 1).Value = runId
    lg.Cells(r, 2).Value = itemName
    lg.Cells(r, 3).Value = Format(Now, "yyyy-mm-dd hh:nn:ss")
    lg.Cells(r, 4).Value = Environ$("USERNAME")
    lg.Cells(r, 5).Value = ActiveWorkbook.Name
    lg.Cells(r, 6).Value = srcSheet
    lg.Cells(r, 7).Value = srcRange
    lg.Cells(r, 8).Value = amtName
    lg.Cells(r, 9).Value = popN
    lg.Cells(r, 10).Value = popTotal
    lg.Cells(r, 11).Value = pm
    lg.Cells(r, 12).Value = conf
    lg.Cells(r, 13).Value = rf
    lg.Cells(r, 14).Value = interval
    lg.Cells(r, 15).Value = seed
    lg.Cells(r, 16).Value = startPt
    lg.Cells(r, 17).Value = nKey
    lg.Cells(r, 18).Value = nPps
    lg.Cells(r, 19).Value = nSel
    lg.Cells(r, 20).Value = basicPrec
    lg.Cells(r, 21).Value = resultSheet
End Sub

Public Sub Auto_Open()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
    Dim cb As CommandBar
    Set cb = Application.CommandBars.Add(Name:="axon MUS", Position:=msoBarTop, Temporary:=True)
    Dim b As CommandBarButton
    Set b = cb.Controls.Add(Type:=msoControlButton)
    b.Caption = "MUS Sample": b.Style = msoButtonCaption: b.OnAction = "MUS_Sample"
    Dim b2 As CommandBarButton
    Set b2 = cb.Controls.Add(Type:=msoControlButton)
    b2.Caption = "MUS Evaluate": b2.Style = msoButtonCaption: b2.OnAction = "MUS_Evaluate"
    cb.Visible = True
End Sub

Public Sub Auto_Close()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
End Sub
