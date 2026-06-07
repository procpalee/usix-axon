Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS - Monetary Unit Sampling (PPS). Pure VBA, ASCII only.
'  No dependencies, no encoding issues on any Windows locale.
' ------------------------------------------------------------
'  How to use:
'   1) Put data on a sheet (top row = header).
'   2) Alt+F8 -> run "MUS_Sample".
'   3) Enter range / amount column / PM / confidence / seed.
'   4) A new sheet shows Key Items + PPS sample with
'      Type / Book_Value / Audit_Value columns.
'  Deterministic: same (range, PM, seed) = same sample.
'  R = -ln(1 - confidence); interval = PM / R.
'  amount >= interval -> Key Item (all selected);
'  the rest -> systematic PPS selection from a seeded start.
' ============================================================

Public Sub MUS_Sample()
    Dim rng As Range
    On Error Resume Next
    Set rng = Application.InputBox( _
        "Select the data range (top row = header).", _
        "MUS - Range", Selection.Address, Type:=8)
    On Error GoTo 0
    If rng Is Nothing Then Exit Sub

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

    s = InputBox("Performance materiality (PM):", "MUS - PM", "1000000")
    If Not IsNumeric(s) Then Exit Sub
    Dim pm As Double: pm = CDbl(s)
    If pm <= 0 Then MsgBox "PM must be positive.", vbExclamation: Exit Sub

    s = InputBox("Confidence (0.90 / 0.95 / 0.99):", "MUS - Confidence", "0.95")
    If Not IsNumeric(s) Then Exit Sub
    Dim conf As Double: conf = CDbl(s)
    If conf < 0.5 Then conf = 0.5
    If conf > 0.999 Then conf = 0.999

    s = InputBox("Seed (same seed = same sample):", "MUS - Seed", "42")
    If Not IsNumeric(s) Then Exit Sub
    Dim seed As Double: seed = CDbl(s)

    Dim interval As Double: interval = pm / (-Log(1# - conf))   ' Log = natural log (ln)

    Dim keyIdx() As Long, ppsIdx() As Long, ppsAmt() As Double
    Dim nKey As Long, nPps As Long, i As Long
    ReDim keyIdx(1 To nRows): ReDim ppsIdx(1 To nRows): ReDim ppsAmt(1 To nRows)
    For i = 2 To nRows
        If IsNumeric(data(i, amtCol)) Then
            Dim a As Double: a = CDbl(data(i, amtCol))
            If a > 0 Then
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
        nSel = nSel + 1: selRow(nSel) = keyIdx(i): selType(nSel) = "Key Item"
    Next i

    If nPps > 0 Then
        Dim cum() As Double, total As Double: ReDim cum(1 To nPps)
        For i = 1 To nPps
            total = total + ppsAmt(i): cum(i) = total
        Next i
        Rnd -1: Randomize seed
        Dim start As Double: start = 0.0001 + Rnd() * (interval - 0.0001)
        Dim cnt As Long, j As Long, last As Long, idx As Long
        cnt = Int((total - start) / interval) + 1
        If cnt < 1 Then cnt = 1
        last = -1
        For j = 0 To cnt - 1
            Dim point As Double: point = start + j * interval
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

    Dim ws As Worksheet: Set ws = ThisWorkbook.Worksheets.Add
    On Error Resume Next
    ws.Name = "MUS " & Format(conf * 100, "0") & "pct s" & seed
    On Error GoTo 0
    For c = 1 To nCols
        ws.Cells(1, c).Value = data(1, c)
    Next c
    ws.Cells(1, nCols + 1).Value = "Type"
    ws.Cells(1, nCols + 2).Value = "Book_Value"
    ws.Cells(1, nCols + 3).Value = "Audit_Value"
    Dim r As Long, srcRow As Long
    For r = 1 To nSel
        srcRow = selRow(r)
        For c = 1 To nCols
            ws.Cells(r + 1, c).Value = data(srcRow, c)
        Next c
        ws.Cells(r + 1, nCols + 1).Value = selType(r)
        ws.Cells(r + 1, nCols + 2).Value = data(srcRow, amtCol)
        ws.Cells(r + 1, nCols + 3).Value = data(srcRow, amtCol)
    Next r
    ws.Columns.AutoFit
    MsgBox "Sample: " & nSel & " (Key " & nKey & " / PPS " & (nSel - nKey) & ")" & _
           vbCrLf & "Sheet: " & ws.Name, vbInformation, "axon MUS"
End Sub
