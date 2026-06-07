Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS - 화폐단위표본추출(PPS) 감사용  (한글 UI)
'  순수 VBA. ISA 530 / AICPA MUS 표본추출 + 평가 준거.
' ------------------------------------------------------------
'  매크로
'   MUS_Sample   - 표본을 뽑아 자기문서화 조서 시트를 만들고
'                  실행 로그를 남긴다.
'   MUS_Evaluate - 감사가(Audit_Value) 입력 후 결과 시트에서 실행.
'                  순위별 증분 신뢰계수로 상한오차(UEL)와 판정을 정밀 재계산.
'
'  계획: R0 = 오류 0건 신뢰계수(포아송), 표본간격 = PM / R0.
'        금액 >= 간격 -> 고액항목(전수), 미만 -> 시드 기반 체계적 추출.
'  평가(AICPA 증분법):
'        기본정밀도 = R0 * 간격
'        오염률 = (장부가-감사가)/장부가, 추정왜곡 = 오염률 * 간격
'        증분계수 incr_j = R(j) - R(j-1)  (오염률 내림차순 순위)
'        UEL = 기본정밀도 + SUM(추정왜곡_j * incr_j) + 고액항목 실액
'        판정: UEL <= PM 이면 수용 가능.
'  결정론: 같은 (범위/PM/신뢰수준/시드) = 같은 표본. 실행마다 MUS_Log 기록.
' ============================================================

Private Const LOG_SHEET As String = "MUS_Log"
Private Const T_KEY As String = "고액항목"
Private Const T_PPS As String = "PPS표본"
Private Const H_TYPE As String = "유형"
Private Const H_BOOK As String = "장부가"
Private Const H_AUDIT As String = "감사가"
Private Const H_TAINT As String = "오염률"
Private Const H_PROJ As String = "추정왜곡"

Public Sub MUS_Sample()
    Dim rng As Range
    On Error Resume Next
    Set rng = Application.InputBox( _
        "데이터 범위를 선택하세요 (맨 윗줄 = 헤더 포함).", _
        "MUS - 범위", Selection.Address, Type:=8)
    On Error GoTo 0
    If rng Is Nothing Then Exit Sub

    Dim srcWs As Worksheet: Set srcWs = rng.Worksheet
    Dim data As Variant: data = rng.Value
    If Not IsArray(data) Then MsgBox "여러 행을 선택하세요.", vbExclamation: Exit Sub
    Dim nRows As Long, nCols As Long
    nRows = UBound(data, 1): nCols = UBound(data, 2)
    If nRows < 2 Then MsgBox "헤더 + 데이터 최소 2행 필요.", vbExclamation: Exit Sub

    Dim headers As String, c As Long
    For c = 1 To nCols
        headers = headers & c & ".  " & data(1, c) & vbCrLf
    Next c
    Dim s As String
    s = InputBox("금액열 번호:" & vbCrLf & vbCrLf & headers, "MUS - 금액열")
    If Not IsNumeric(s) Then Exit Sub
    Dim amtCol As Long: amtCol = CLng(s)
    If amtCol < 1 Or amtCol > nCols Then MsgBox "1 ~ " & nCols & " 범위로 입력하세요.", vbExclamation: Exit Sub

    s = InputBox("수행중요성 (PM / 허용왜곡표시):", "MUS - PM", "1000000")
    If Not IsNumeric(s) Then Exit Sub
    Dim pm As Double: pm = CDbl(s)
    If pm <= 0 Then MsgBox "PM 은 양수여야 합니다.", vbExclamation: Exit Sub

    s = InputBox("신뢰수준 (0.90 / 0.95 / 0.99):", "MUS - 신뢰수준", "0.95")
    If Not IsNumeric(s) Then Exit Sub
    Dim conf As Double: conf = CDbl(s)
    If conf < 0.5 Then conf = 0.5
    If conf > 0.999 Then conf = 0.999

    s = InputBox("시드 (같은 시드 = 같은 표본, 재현용):", "MUS - 시드", "42")
    If Not IsNumeric(s) Then Exit Sub
    Dim seed As Double: seed = CDbl(s)

    Dim rf As Double: rf = ReliabilityFactor(conf, 0)   ' R0
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

    If nSel = 0 Then MsgBox "표본이 선택되지 않았습니다. PM / 금액열 확인.", vbExclamation: Exit Sub
    Dim basicPrec As Double: basicPrec = interval * rf

    Dim ws As Worksheet: Set ws = ActiveWorkbook.Worksheets.Add
    Dim runId As Long: runId = NextRunId()
    On Error Resume Next
    ws.Name = "MUS run" & runId
    On Error GoTo 0

    Dim HR As Long: HR = 26
    ws.Range("A1").Value = "axon MUS - 감사조서"
    PutP ws, 2, "실행ID", runId
    PutP ws, 3, "실행시각", Format(Now, "yyyy-mm-dd hh:nn:ss")
    PutP ws, 4, "수행자", Environ$("USERNAME")
    PutP ws, 5, "통합문서", ActiveWorkbook.Name
    PutP ws, 6, "원본시트", srcWs.Name
    PutP ws, 7, "원본범위", rng.Address(False, False)
    PutP ws, 8, "금액열", CStr(data(1, amtCol))
    PutP ws, 9, "모집단 건수(양수)", popN
    PutP ws, 10, "모집단 총액", popTotal
    PutP ws, 11, "수행중요성(PM)", pm
    PutP ws, 12, "신뢰수준", conf
    PutP ws, 13, "신뢰계수 R0", rf
    PutP ws, 14, "표본간격", interval
    PutP ws, 15, "시드", seed
    PutP ws, 16, "랜덤시작점", startPt
    PutP ws, 17, "고액항목 수", nKey
    PutP ws, 18, "PPS표본 수", nSel - nKey
    PutP ws, 19, "표본 크기", nSel
    PutP ws, 20, "기본정밀도", basicPrec
    PutP ws, 21, "추정왜곡표시", 0
    PutP ws, 22, "증분허용오차", 0
    PutP ws, 23, "상한오차(UEL)", basicPrec
    PutP ws, 24, "판정 (감사가 입력 후 MUS_Evaluate 실행)", ""

    For c = 1 To nCols
        ws.Cells(HR, c).Value = data(1, c)
    Next c
    ws.Cells(HR, nCols + 1).Value = H_TYPE
    ws.Cells(HR, nCols + 2).Value = H_BOOK
    ws.Cells(HR, nCols + 3).Value = H_AUDIT
    ws.Cells(HR, nCols + 4).Value = H_TAINT
    ws.Cells(HR, nCols + 5).Value = H_PROJ

    Dim r As Long, srcRow As Long, rowAbs As Long
    For r = 1 To nSel
        srcRow = selRow(r)
        rowAbs = HR + r
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
            ws.Cells(rowAbs, nCols + 5).FormulaR1C1 = "=RC[-1]*R14C2"
        End If
    Next r

    ws.Columns.AutoFit
    ws.Range("A1").Font.Bold = True
    ws.Rows(HR).Font.Bold = True

    EvaluateSheet ws

    WriteLog runId, srcWs.Name, rng.Address(False, False), CStr(data(1, amtCol)), _
             popN, popTotal, pm, conf, rf, interval, seed, startPt, _
             nKey, nSel - nKey, nSel, basicPrec, ws.Name

    Application.Goto ws.Range("A1"), True
    MsgBox "표본 " & nSel & "건 (고액 " & nKey & " / PPS " & (nSel - nKey) & ")" & vbCrLf & _
           "시트: " & ws.Name & "   로그: run " & runId & " ('" & LOG_SHEET & "')." & vbCrLf & vbCrLf & _
           "다음: 감사가 열을 입력한 뒤 MUS_Evaluate 로 정밀 UEL 을 계산하세요.", _
           vbInformation, "axon MUS"
End Sub

Public Sub MUS_Evaluate()
    On Error GoTo fail
    EvaluateSheet ActiveSheet
    Application.Goto ActiveSheet.Range("A20"), True
    MsgBox "평가 갱신됨 (B21..B24)." & vbCrLf & _
           "UEL 은 순위별 증분 신뢰계수를 적용합니다." & vbCrLf & _
           "UEL <= PM 이면 수용 가능, 아니면 추가 절차가 필요합니다.", _
           vbInformation, "axon MUS - 평가"
    Exit Sub
fail:
    MsgBox "MUS_Sample 로 만든 결과 시트에서 실행하세요.", vbExclamation
End Sub

Private Sub EvaluateSheet(ws As Worksheet)
    Dim pm As Double: pm = ws.Range("B11").Value
    Dim conf As Double: conf = ws.Range("B12").Value
    Dim r0 As Double: r0 = ws.Range("B13").Value
    Dim interval As Double: interval = ws.Range("B14").Value

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
        Else
            If book > 0 Then
                taint = (book - audit) / book
                If taint > 0 Then
                    m = m + 1
                    ReDim Preserve proj(1 To m)
                    proj(m) = taint * interval
                End If
            End If
        End If
        r = r + 1
    Loop

    Dim i As Long, j As Long, tmp As Double
    For i = 1 To m - 1
        For j = 1 To m - i
            If proj(j) < proj(j + 1) Then
                tmp = proj(j): proj(j) = proj(j + 1): proj(j + 1) = tmp
            End If
        Next j
    Next i

    Dim basicPrec As Double: basicPrec = r0 * interval
    Dim rankedSum As Double, projPPS As Double, incr As Double
    Dim rPrev As Double, rCur As Double
    rPrev = r0
    For j = 1 To m
        rCur = ReliabilityFactor(conf, j)
        incr = rCur - rPrev
        rankedSum = rankedSum + proj(j) * incr
        projPPS = projPPS + proj(j)
        rPrev = rCur
    Next j

    Dim uel As Double: uel = basicPrec + rankedSum + keyActual
    Dim projectedTotal As Double: projectedTotal = projPPS + keyActual
    Dim incrementalAllow As Double: incrementalAllow = rankedSum - projPPS

    ws.Range("B20").Value = basicPrec
    ws.Range("B21").Value = projectedTotal
    ws.Range("B22").Value = incrementalAllow
    ws.Range("B23").Value = uel
    ws.Range("A24").Value = "판정"
    If uel <= pm Then
        ws.Range("B24").Value = "수용 가능 (UEL <= PM)"
    Else
        ws.Range("B24").Value = "추가 절차 필요 (UEL > PM)"
    End If
End Sub

Private Function ReliabilityFactor(ByVal conf As Double, ByVal k As Long) As Double
    Dim target As Double: target = 1# - conf
    Dim lo As Double, hi As Double, mid As Double, it As Long
    lo = 0#: hi = 50# + 2# * k
    For it = 1 To 200
        mid = (lo + hi) / 2#
        If PoissonCDF(k, mid) > target Then
            lo = mid
        Else
            hi = mid
        End If
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

Private Function SafeNum(v As Variant) As Double
    If IsNumeric(v) Then SafeNum = CDbl(v) Else SafeNum = 0#
End Function

Private Sub PutP(ws As Worksheet, rowN As Long, label As String, v As Variant)
    ws.Cells(rowN, 1).Value = label
    ws.Cells(rowN, 2).Value = v
End Sub

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
        hdr = Array("실행ID", "시각", "사용자", "통합문서", "원본시트", _
            "원본범위", "금액열", "모집단건수", "모집단총액", "PM", _
            "신뢰수준", "신뢰계수", "표본간격", "시드", "랜덤시작점", _
            "고액항목", "PPS표본", "표본크기", "기본정밀도", "결과시트")
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

Public Sub Auto_Open()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
    Dim cb As CommandBar
    Set cb = Application.CommandBars.Add(Name:="axon MUS", Position:=msoBarTop, Temporary:=True)
    Dim b As CommandBarButton
    Set b = cb.Controls.Add(Type:=msoControlButton)
    b.Caption = "MUS 표본추출": b.Style = msoButtonCaption: b.OnAction = "MUS_Sample"
    Dim b2 As CommandBarButton
    Set b2 = cb.Controls.Add(Type:=msoControlButton)
    b2.Caption = "MUS 평가": b2.Style = msoButtonCaption: b2.OnAction = "MUS_Evaluate"
    cb.Visible = True
End Sub

Public Sub Auto_Close()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
End Sub
