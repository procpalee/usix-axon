Attribute VB_Name = "MUS"
Option Explicit

' ============================================================
'  axon MUS - 화폐단위표본추출(PPS) 감사용 (한글 UI)
'  순수 VBA. ISA 530 / AICPA MUS 표본추출 + 평가 준거.
' ------------------------------------------------------------
'  MUS_Sample   - 표본을 뽑아 표를 만들고, 요약을 A1 셀 노트로 첨부하고,
'                 실행 로그를 남긴다.
'  MUS_Evaluate - 감사가(Audit_Value) 입력 후 실행. 순위별 증분 신뢰계수로
'                 상한오차(UEL)를 정밀 재계산하고 노트를 갱신.
'  R(k) = 포아송 신뢰계수(순수 VBA). 표본간격 = PM / R0.
' ============================================================

Private Const LOG_SHEET As String = "MUS_Log"
Private Const T_KEY As String = "고액항목"
Private Const T_PPS As String = "PPS표본"
Private Const H_TYPE As String = "유형"
Private Const H_BOOK As String = "장부가"
Private Const H_AUDIT As String = "감사가"
Private Const H_TAINT As String = "오염률"
Private Const H_PROJ As String = "추정왜곡"
Private Const MARKER As String = "===== 평가 ====="
Private Const HELP_SHEET As String = "MUS_사용법"

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

    Dim itemName As String
    itemName = Trim(InputBox("표본추출 항목 이름 (시트명에 사용, 예: 매출채권):", "MUS - 항목명", "표본"))
    If itemName = "" Then itemName = "표본"

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

    If nSel = 0 Then MsgBox "표본이 선택되지 않았습니다. PM / 금액열 확인.", vbExclamation: Exit Sub
    Dim basicPrec As Double: basicPrec = interval * rf

    Dim ws As Worksheet: Set ws = ActiveWorkbook.Worksheets.Add
    Dim runId As Long: runId = NextRunId()
    NameSheet ws, itemName, runId
    ws.Tab.Color = RGB(46, 117, 182)

    AddNum ws, "MUS_PM", pm
    AddNum ws, "MUS_CONF", conf
    AddNum ws, "MUS_INTERVAL", interval

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

    ws.Range(ws.Cells(2, nCols + 2), ws.Cells(1 + nSel, nCols + 3)).NumberFormat = "#,##0"
    ws.Range(ws.Cells(2, nCols + 4), ws.Cells(1 + nSel, nCols + 4)).NumberFormat = "0.0%"
    ws.Range(ws.Cells(2, nCols + 5), ws.Cells(1 + nSel, nCols + 5)).NumberFormat = "#,##0"
    ws.Rows(1).Font.Bold = True
    ws.Columns.AutoFit

    Dim st As String
    st = "MUS Sampling Summary" & vbLf & String(34, "-") & vbLf
    st = st & "항목: " & itemName & vbLf
    st = st & "실행ID: " & runId & "    실행시각: " & Format(Now, "yyyy-mm-dd hh:nn:ss") & vbLf
    st = st & "수행자: " & Environ$("USERNAME") & vbLf
    st = st & "통합문서: " & ActiveWorkbook.Name & vbLf
    st = st & "원본: " & srcWs.Name & "!" & rng.Address(False, False) & "   금액열: " & CStr(data(1, amtCol)) & vbLf & vbLf
    st = st & "[계획]" & vbLf
    st = st & "모집단: " & Fmt(popN) & "건 / " & Fmt(popTotal) & vbLf
    st = st & "PM(허용왜곡): " & Fmt(pm) & vbLf
    st = st & "신뢰수준: " & Format(conf * 100, "0") & "%" & vbLf
    st = st & "신뢰계수 R0: " & Fmt(rf) & vbLf
    st = st & "표본간격: " & Fmt(interval) & vbLf
    st = st & "시드: " & Fmt(seed) & "   랜덤시작점: " & Fmt(startPt) & vbLf
    st = st & "표본: " & Fmt(nSel) & "  (고액 " & Fmt(nKey) & " / PPS " & Fmt(nSel - nKey) & ")" & vbLf
    st = st & "기본정밀도: " & Fmt(basicPrec) & vbLf & vbLf

    SetNote ws, st & MARKER & vbLf & "(대기 - 감사가 입력 후 MUS_Evaluate 실행)"
    EvaluateSheet ws

    WriteLog runId, itemName, srcWs.Name, rng.Address(False, False), CStr(data(1, amtCol)), _
             popN, popTotal, pm, conf, rf, interval, seed, startPt, _
             nKey, nSel - nKey, nSel, basicPrec, ws.Name

    Application.Goto ws.Range("A1"), True
    MsgBox "표본 " & nSel & "건 (고액 " & nKey & " / PPS " & (nSel - nKey) & ")" & vbCrLf & _
           "시트: " & ws.Name & "   (요약은 A1 셀의 노트)" & vbCrLf & _
           "로그: run " & runId & " ('" & LOG_SHEET & "')." & vbCrLf & vbCrLf & _
           "다음: 감사가 열을 입력한 뒤 MUS_Evaluate 로 정밀 UEL 을 계산하세요.", _
           vbInformation, "axon MUS"
End Sub

Public Sub MUS_Evaluate()
    On Error GoTo fail
    EvaluateSheet ActiveSheet
    MsgBox "A1 노트의 평가가 갱신되었습니다." & vbCrLf & _
           "UEL 은 순위별 증분 신뢰계수를 적용합니다." & vbCrLf & _
           "UEL <= PM 이면 수용 가능, 아니면 추가 절차가 필요합니다.", _
           vbInformation, "axon MUS - 평가"
    Exit Sub
fail:
    MsgBox "MUS_Sample 로 만든 결과 시트에서 실행하세요.", vbExclamation
End Sub

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
    If uel <= pm Then verdict = "수용 가능 (UEL <= PM)" Else verdict = "추가 절차 필요 (UEL > PM)"

    Dim ev As String
    ev = MARKER & vbLf
    ev = ev & "추정왜곡표시: " & Fmt(projectedTotal) & vbLf
    ev = ev & "증분허용오차: " & Fmt(incrementalAllow) & vbLf
    ev = ev & "상한오차(UEL): " & Fmt(uel) & vbLf
    ev = ev & "판정: " & verdict

    Dim full As String, pos As Long
    full = GetNote(ws)
    pos = InStr(full, MARKER)
    If pos > 0 Then full = Left$(full, pos - 1)
    SetNote ws, full & ev
End Sub

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
        hdr = Array("실행ID", "항목", "시각", "사용자", "통합문서", "원본시트", _
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

' --- 추가기능 도움말: "MUS_사용법" 시트에 상세 사용법 생성 ---
Public Sub MUS_Help()
    Dim ws As Worksheet
    On Error Resume Next
    Set ws = ActiveWorkbook.Worksheets(HELP_SHEET)
    On Error GoTo 0
    If ws Is Nothing Then
        Set ws = ActiveWorkbook.Worksheets.Add
        ws.Name = HELP_SHEET
    Else
        ws.Cells.Clear
    End If
    ws.Tab.Color = RGB(112, 173, 71)

    Dim L As Long: L = 1
    HL ws, L, "axon MUS - 사용법", True
    HL ws, L, "", False
    HL ws, L, "1. 개요", True
    HL ws, L, "MUS(화폐단위표본추출, PPS)는 금액이 큰 거래일수록 뽑힐 확률이 높은 감사 표본추출 기법입니다. 모든 수치 계산은 로컬에서 결정론으로 처리됩니다.", False
    HL ws, L, "", False
    HL ws, L, "2. 준비", True
    HL ws, L, "분석할 데이터를 시트에 두고, 맨 윗줄은 반드시 제목(헤더) 행이어야 합니다. 금액 열에는 숫자만 두세요(양수만 모집단).", False
    HL ws, L, "", False
    HL ws, L, "3. 표본추출  -  [MUS 표본추출] 버튼", True
    HL ws, L, "순서대로 묻습니다: (1) 데이터 범위  (2) 금액열 번호  (3) 수행중요성 PM  (4) 신뢰수준 0.90/0.95/0.99  (5) 시드  (6) 항목 이름.", False
    HL ws, L, "실행하면 'MUS_<항목>' 시트가 생기고 탭이 파란색으로 표시됩니다. A1 셀의 노트에 요약(파라미터/표본수)이 들어가고, MUS_Log 시트에 실행 한 줄이 기록됩니다.", False
    HL ws, L, "결과 표 열: 유형(고액항목=전수검사 / PPS표본=체계추출), 장부가, 감사가(직접 입력), 오염률, 추정왜곡.", False
    HL ws, L, "", False
    HL ws, L, "4. 평가  -  [MUS 평가] 버튼", True
    HL ws, L, "표본 항목을 검토해 실제 금액을 '감사가' 열에 입력합니다. 그다음 [MUS 평가]를 누르면 A1 노트의 추정왜곡/상한오차(UEL)/판정이 다시 계산됩니다.", False
    HL ws, L, "판정: UEL <= PM 이면 '수용 가능', 크면 '추가 절차 필요'. 감사가를 수정할 때마다 [MUS 평가]를 다시 누르세요.", False
    HL ws, L, "", False
    HL ws, L, "5. 주요 용어", True
    HL ws, L, "PM(수행중요성): 이 금액까지 틀려도 결론에 영향 없다는 한계선(허용왜곡표시).", False
    HL ws, L, "신뢰수준: 결론의 확신 정도(95% 권장). 높을수록 표본이 많아집니다.", False
    HL ws, L, "표본간격 = PM / 신뢰계수(R). 이 간격 이상 금액은 고액항목으로 전수검사.", False
    HL ws, L, "기본정밀도 = 표본간격 x R. 오류가 없을 때의 상한오차.", False
    HL ws, L, "오염률 = (장부가 - 감사가) / 장부가.  추정왜곡 = 오염률 x 표본간격.", False
    HL ws, L, "상한오차(UEL) = 기본정밀도 + 순위별 증분 신뢰계수로 가중한 추정왜곡 합 + 고액항목 실차이.", False
    HL ws, L, "", False
    HL ws, L, "6. 재현성 / 로그", True
    HL ws, L, "같은 (범위/PM/신뢰수준/시드)면 항상 같은 표본이 나옵니다. 시드와 랜덤시작점이 노트와 MUS_Log에 기록되어 재수행 시 동일 표본을 재현할 수 있습니다.", False
    HL ws, L, "MUS_Log 시트는 통합문서의 모든 실행 이력을 한 줄씩 쌓는 누적 감사추적 대장입니다.", False
    HL ws, L, "", False
    HL ws, L, "7. 주의", True
    HL ws, L, "난수는 엑셀 Rnd(시드 기반)이라 도구 내 재현은 보장되지만 IDEA/ACL 등 타 도구와 표본 숫자가 동일하지는 않습니다.", False
    HL ws, L, "실제 감사증거로 쓰기 전 소속 법인의 감사방법론 부합 여부를 검토하세요.", False

    ws.Columns(1).ColumnWidth = 95
    ws.Range(ws.Cells(1, 1), ws.Cells(L - 1, 1)).WrapText = True
    ws.Range("A1").Font.Size = 14
    ws.Rows.AutoFit
    ws.Activate
    ws.Range("A1").Select
End Sub

Private Sub HL(ws As Worksheet, ByRef L As Long, text As String, bold As Boolean)
    ws.Cells(L, 1).Value = text
    ws.Cells(L, 1).Font.bold = bold
    L = L + 1
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
    Dim b3 As CommandBarButton
    Set b3 = cb.Controls.Add(Type:=msoControlButton)
    b3.Caption = "MUS 도움말": b3.Style = msoButtonCaption: b3.OnAction = "MUS_Help"
    cb.Visible = True
End Sub

Public Sub Auto_Close()
    On Error Resume Next
    Application.CommandBars("axon MUS").Delete
    On Error GoTo 0
End Sub
