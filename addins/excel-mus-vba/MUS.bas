Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS — 화폐단위표본추출 (PPS)  ·  순수 VBA, 설치 의존성 0
' ------------------------------------------------------------
'  사용법
'   1) 데이터(맨 윗줄 = 제목 헤더)를 시트에 둔다.
'   2) Alt+F8 → "MUS_표본추출" 실행 (또는 버튼에 연결).
'   3) 범위·금액열·PM·신뢰수준·시드를 입력하면
'      새 시트에 Key Item(고액 전수) + PPS 표본을 출력한다.
'
'  결정론: 같은 (범위·PM·시드) = 항상 같은 표본 (감사조서 재현용).
'  ※ R(신뢰계수) = -ln(1 - 신뢰수준), 기대오류 0건 기준 포아송.
'    interval = PM / R. interval 이상 = Key Item 전수,
'    나머지는 시드 기반 시작점에서 interval 간격 체계적 추출.
' ============================================================

Public Sub MUS_표본추출()
    Dim rng As Range

    ' --- 1. 분석 범위 선택 (현재 선택을 기본값으로) ---
    On Error Resume Next
    Set rng = Application.InputBox( _
        "분석할 데이터 범위를 선택하세요 (맨 윗줄 = 제목 헤더 포함).", _
        "MUS — 범위 선택", Selection.Address, Type:=8)
    On Error GoTo 0
    If rng Is Nothing Then Exit Sub   ' 취소

    Dim data As Variant
    data = rng.Value
    If Not IsArray(data) Then
        MsgBox "범위를 더 넓게(여러 행) 선택하세요.", vbExclamation: Exit Sub
    End If

    Dim nRows As Long, nCols As Long
    nRows = UBound(data, 1)
    nCols = UBound(data, 2)
    If nRows < 2 Then
        MsgBox "헤더행 + 데이터행이 최소 2행 필요합니다.", vbExclamation: Exit Sub
    End If

    ' --- 2. 금액열 선택 (헤더 목록 → 번호 입력) ---
    Dim headers As String, c As Long
    For c = 1 To nCols
        headers = headers & c & ".  " & data(1, c) & vbCrLf
    Next c
    Dim s As String
    s = InputBox("금액열 '번호'를 입력하세요:" & vbCrLf & vbCrLf & headers, "MUS — 금액열")
    If Not IsNumeric(s) Then Exit Sub
    Dim amtCol As Long: amtCol = CLng(s)
    If amtCol < 1 Or amtCol > nCols Then
        MsgBox "1 ~ " & nCols & " 사이의 번호를 입력하세요.", vbExclamation: Exit Sub
    End If

    ' --- 3. PM · 신뢰수준 · 시드 ---
    s = InputBox("수행중요성 (PM) 금액:", "MUS — PM", "1000000")
    If Not IsNumeric(s) Then Exit Sub
    Dim pm As Double: pm = CDbl(s)
    If pm <= 0 Then MsgBox "PM 은 양수여야 합니다.", vbExclamation: Exit Sub

    s = InputBox("신뢰수준 (0.90 / 0.95 / 0.99):", "MUS — 신뢰수준", "0.95")
    If Not IsNumeric(s) Then Exit Sub
    Dim conf As Double: conf = CDbl(s)
    If conf < 0.5 Then conf = 0.5
    If conf > 0.999 Then conf = 0.999

    s = InputBox("시드 (재현용 숫자 — 같은 시드 = 같은 표본):", "MUS — 시드", "42")
    If Not IsNumeric(s) Then Exit Sub
    Dim seed As Double: seed = CDbl(s)

    ' --- 4. R-factor · interval ---
    Dim interval As Double
    interval = pm / (-Log(1# - conf))          ' VBA Log = 자연로그(ln)

    ' --- 5. 모집단(양수)에서 Key Item / PPS 분리 ---
    Dim keyIdx() As Long, ppsIdx() As Long, ppsAmt() As Double
    Dim nKey As Long, nPps As Long, i As Long
    ReDim keyIdx(1 To nRows): ReDim ppsIdx(1 To nRows): ReDim ppsAmt(1 To nRows)

    For i = 2 To nRows                          ' 2행부터 = 데이터
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

    ' --- 6. 선택 누적: Key Item 전수 먼저 ---
    Dim selRow() As Long, selType() As String, nSel As Long
    ReDim selRow(1 To nRows): ReDim selType(1 To nRows)
    For i = 1 To nKey
        nSel = nSel + 1: selRow(nSel) = keyIdx(i): selType(nSel) = "Key Item"
    Next i

    ' --- 7. PPS 체계적 추출 ---
    If nPps > 0 Then
        Dim cum() As Double, total As Double
        ReDim cum(1 To nPps)
        For i = 1 To nPps
            total = total + ppsAmt(i): cum(i) = total
        Next i

        ' 결정론 시작점: 시드 고정 → 재현 가능.
        Rnd -1
        Randomize seed
        Dim start As Double
        start = 0.0001 + Rnd() * (interval - 0.0001)

        Dim cnt As Long, j As Long, last As Long, idx As Long
        cnt = Int((total - start) / interval) + 1
        If cnt < 1 Then cnt = 1
        last = -1
        For j = 0 To cnt - 1
            Dim point As Double: point = start + j * interval
            idx = 0
            For i = 1 To nPps                   ' 첫 cum >= point
                If cum(i) >= point Then idx = i: Exit For
            Next i
            If idx >= 1 And idx <> last Then
                nSel = nSel + 1
                selRow(nSel) = ppsIdx(idx): selType(nSel) = "PPS Sample"
                last = idx
            End If
        Next j
    End If

    If nSel = 0 Then
        MsgBox "선택된 표본이 없습니다. PM / 금액열을 확인하세요.", vbExclamation: Exit Sub
    End If

    ' --- 8. 새 시트에 출력 (원본열 + Type/Book_Value/Audit_Value) ---
    Dim ws As Worksheet
    Set ws = ThisWorkbook.Worksheets.Add
    On Error Resume Next
    ws.Name = "MUS " & Format(conf * 100, "0") & "% s" & seed   ' 시드 박힌 시트명
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
        ws.Cells(r + 1, nCols + 2).Value = data(srcRow, amtCol)  ' 장부가
        ws.Cells(r + 1, nCols + 3).Value = data(srcRow, amtCol)  ' 감사가(=장부가, 이후 수정)
    Next r
    ws.Columns.AutoFit

    MsgBox "표본 " & nSel & "건 추출 완료." & vbCrLf & _
           "Key Item: " & nKey & "  ·  PPS: " & (nSel - nKey) & vbCrLf & _
           "결과 시트: " & ws.Name, vbInformation, "axon MUS"
End Sub
