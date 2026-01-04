use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            LibraryLoader::*,
            Threading::*,
        },
        UI::Input::KeyboardAndMouse::*,
        UI::WindowsAndMessaging::*,
    },
};

static mut RUNNING: bool = true;
static mut OVERLAY_ENABLED: bool = true;

// ===============================
// util
// ===============================
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

// ===============================
// overlay window proc
// ===============================
extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_HOTKEY => {
                OVERLAY_ENABLED = !OVERLAY_ENABLED;
                let _ = ShowWindow(hwnd, if OVERLAY_ENABLED { SW_SHOW } else { SW_HIDE });
                LRESULT(0)
            }
            WM_DESTROY => {
                RUNNING = false;
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_PAINT => {
                if !OVERLAY_ENABLED {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }

                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                // 1. 전체 영역 확보
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);

                // 2. 배경 지우기
                let hbr = CreateSolidBrush(rgb(0, 0, 0));
                let _ = FillRect(hdc, &rect, hbr);
                let _ = DeleteObject(hbr);

                // 3. 텍스트 설정
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, rgb(255, 0, 0));

                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                // 해상도의 5% 지점을 상단 시작점으로 설정 (비율에 따라 자동 조절)
                rect.top = height / 20; 

                let display_text = format!(
                    "Maple Overlay ON (Ctrl + F1)\nResolution: {}x{}",
                    width, height
                );
                let mut text: Vec<u16> = display_text.encode_utf16().collect();

                // 4. 중앙 상단 정렬하여 그리기
                // DT_CENTER: 가로 중앙
                // DT_VCENTER를 제거하여 상단에 고정하고, \n(줄바꿈) 인식을 위해 DT_SINGLELINE도 제거합니다.
                let _ = DrawTextW(
                    hdc,
                    &mut text,
                    &mut rect,
                    DT_CENTER,
                );

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ===============================
// EnumWindows helper
// ===============================
struct FindResult {
    hwnd: Option<HWND>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let result = &mut *(lparam.0 as *mut FindResult);

    // 1. 기본 가시성 체크
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL::from(true);
    }

    // 2. 윈도우 클래스 이름 확인 (외부 채팅창과 구분하는 핵심)
    let mut class_buffer = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buffer);
    let class_name = String::from_utf16_lossy(&class_buffer[..class_len as usize]);

    // 메이플스토리 본체는 반드시 "MapleStoryClass"라는 이름을 가집니다.
    if class_name != "MapleStoryClass" {
        return BOOL::from(true);
    }

    // 3. 윈도우 스타일 확인
    // 본체 게임창은 일반적으로 WS_VISIBLE이 켜져 있고, 소유주(Owner)가 없는 최상위 창입니다.
    let owner = GetWindow(hwnd, GW_OWNER);
    
    // windows-rs 0.52에서는 HWND의 내부 값이 0이면 소유주가 없는 것입니다.
    if owner.0 != 0 {
        // 소유주가 있는 창(채팅창 등)은 본체일 확률이 낮으므로 패스
        return BOOL::from(true);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == 0 {
        return BOOL::from(true);
    }

    // 4. 프로세스 실행 파일명 최종 확인
    let handle = match OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        false,
        pid,
    ) {
        Ok(h) => h,
        Err(_) => return BOOL::from(true),
    };

    let mut path = [0u16; 260];
    let mut size = path.len() as u32;

    let ok = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        PWSTR(path.as_mut_ptr()),
        &mut size,
    ).is_ok();

    let _ = CloseHandle(handle);

    if ok {
        let full_path = String::from_utf16_lossy(&path[..size as usize]).to_lowercase();
        if full_path.ends_with("maplestory.exe") {
            result.hwnd = Some(hwnd);
            return BOOL::from(false); // 정확한 본체 창을 찾았으므로 중단
        }
    }

    BOOL::from(true)
}

unsafe fn find_maplestory_window() -> Option<HWND> {
    let mut result = FindResult { hwnd: None };

    let _ = EnumWindows(
        Some(enum_windows_proc),
        LPARAM(&mut result as *mut _ as isize),
    );

    result.hwnd
}

// ===============================
// main
// ===============================
fn main() -> Result<()> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();

        // -------------------------------
        // register overlay window
        // -------------------------------
        let class_name = w!("MapleOverlayWindow");

        let wc = WNDCLASSW {
            hInstance: hinstance,
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };

        RegisterClassW(&wc);

        let overlay_hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST,
            class_name,
            w!(""),
            WS_POPUP,
            0,
            0,
            100,
            100,
            None,
            None,
            hinstance,
            None,
        );

        let _ = SetLayeredWindowAttributes(
            overlay_hwnd,
            rgb(0, 0, 0), // 검은색을 투명하게 처리
            255,
            LWA_COLORKEY,
        );

        let _ = ShowWindow(overlay_hwnd, SW_SHOW);

        // Ctrl + F1
        let _ = RegisterHotKey(
            overlay_hwnd,
            1,
            HOT_KEY_MODIFIERS(MOD_CONTROL.0),
            VK_F1.0 as u32,
        );

        // -------------------------------
        // message loop
        // -------------------------------
        let mut msg = MSG::default();

        while RUNNING {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if OVERLAY_ENABLED {
                if let Some(maple_hwnd) = find_maplestory_window() {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(maple_hwnd, &mut rect);

                    // 1. 창이 실제로 화면에 보이는 상태인지 정밀 확인
                    // IsIconic은 창이 최소화되었는지 확인합니다.
                    let is_minimized = IsIconic(maple_hwnd).as_bool();
                    let is_visible = IsWindowVisible(maple_hwnd).as_bool();

                    // 2. 좌표가 유효한지 확인 (최소화되면 보통 비정상적인 좌표를 가짐)
                    let has_valid_size = rect.right > rect.left && rect.bottom > rect.top;
                    
                    if is_visible && !is_minimized && has_valid_size {
                        let _ = ShowWindow(overlay_hwnd, SW_SHOWNOACTIVATE);
                        let _ = MoveWindow(
                            overlay_hwnd,
                            rect.left,
                            rect.top,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                            true,
                        );
                        // InvalidateRect의 세 번째 인자를 true로 설정하여 배경을 다시 그리도록 강제합니다.
                        let _ = InvalidateRect(overlay_hwnd, None, BOOL::from(true));
                    } else {
                        // 창이 최소화되었거나 숨겨졌으면 오버레이도 숨김
                        let _ = ShowWindow(overlay_hwnd, SW_HIDE);
                    }
                } else {
                    // 메이플스토리 창을 찾을 수 없으면(꺼졌으면) 오버레이 숨김
                    let _ = ShowWindow(overlay_hwnd, SW_HIDE);
                }
            }

            Sleep(16); // ~60 FPS
        }
    }

    Ok(())
}
