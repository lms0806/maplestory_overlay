use lazy_static::lazy_static;
use std::sync::Mutex;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{LibraryLoader::*, Threading::*},
        UI::Input::KeyboardAndMouse::*,
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

// 핫키 및 컨트롤 ID 상수
const HOTKEY_ID_TOGGLE: i32 = 1;
const HOTKEY_ID_NICKNAME: i32 = 2;
const ID_BUTTON_OK: isize = 101;

lazy_static! {
    static ref APP_STATE: Mutex<AppState> = Mutex::new(AppState::default());
}

struct AppState {
    running: bool,
    overlay_enabled: bool,
    nickname: String,
    api_key: String,      // API 키 필드 추가
    input_hwnd: HWND,
    edit_hwnd: HWND,
    api_edit_hwnd: HWND,  // API 키 입력용 핸들 추가
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            overlay_enabled: true,
            nickname: String::new(),
            api_key: String::new(),
            input_hwnd: HWND(0),
            edit_hwnd: HWND(0),
            api_edit_hwnd: HWND(0),
        }
    }
}

// ===============================
// util
// ===============================
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

// ===============================
// input window (입력 창)
// ===============================
extern "system" fn input_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND => {
                if (wparam.0 & 0xffff) as isize == ID_BUTTON_OK {
                    let mut state = APP_STATE.lock().unwrap();
                    
                    // 닉네임 읽기
                    let mut nick_buffer = [0u16; 64];
                    let nick_len = GetWindowTextW(state.edit_hwnd, &mut nick_buffer);
                    state.nickname = String::from_utf16_lossy(&nick_buffer[..nick_len as usize]).trim().to_string();

                    // API 키 읽기
                    let mut api_buffer = [0u16; 128];
                    let api_len = GetWindowTextW(state.api_edit_hwnd, &mut api_buffer);
                    state.api_key = String::from_utf16_lossy(&api_buffer[..api_len as usize]).trim().to_string();

                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn ensure_input_box(hinstance: HINSTANCE) {
    let mut state = APP_STATE.lock().unwrap();
    if state.input_hwnd.0 != 0 {
        let _ = ShowWindow(state.input_hwnd, SW_SHOW);
        let _ = SetForegroundWindow(state.input_hwnd);
        return;
    }

    let class_name = w!("SettingsInputWindow");
    let wc = WNDCLASSW {
        hInstance: hinstance,
        lpfnWndProc: Some(input_wnd_proc),
        lpszClassName: class_name,
        hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
        ..Default::default()
    };

    RegisterClassW(&wc);

    state.input_hwnd = CreateWindowExW(
        WS_EX_TOPMOST,
        class_name,
        w!("Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        300,
        220, // 높이를 조금 늘림
        None,
        None,
        hinstance,
        None,
    );

    // 닉네임 라벨 및 입력창
    CreateWindowExW(Default::default(), w!("STATIC"), w!("Nickname:"), WS_CHILD | WS_VISIBLE, 20, 15, 240, 20, state.input_hwnd, None, hinstance, None);
    state.edit_hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        20, 35, 240, 25,
        state.input_hwnd,
        None,
        hinstance,
        None,
    );

    // API 키 라벨 및 입력창
    CreateWindowExW(Default::default(), w!("STATIC"), w!("Open API Key:"), WS_CHILD | WS_VISIBLE, 20, 75, 240, 20, state.input_hwnd, None, hinstance, None);
    state.api_edit_hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        20, 95, 240, 25,
        state.input_hwnd,
        None,
        hinstance,
        None,
    );

    CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        w!("OK"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        100, 140, 80, 30,
        state.input_hwnd,
        HMENU(ID_BUTTON_OK),
        hinstance,
        None,
    );

    let _ = ShowWindow(state.input_hwnd, SW_SHOW);
    let _ = SetForegroundWindow(state.input_hwnd);
}

// ===============================
// overlay window
// ===============================
extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_HOTKEY => {
                match wparam.0 as i32 {
                    HOTKEY_ID_TOGGLE => {
                        let mut state = APP_STATE.lock().unwrap();
                        state.overlay_enabled = !state.overlay_enabled;
                        let _ = ShowWindow(
                            hwnd,
                            if state.overlay_enabled {
                                SW_SHOW
                            } else {
                                SW_HIDE
                            },
                        );
                    }
                    HOTKEY_ID_NICKNAME => {
                        let hinstance = GetModuleHandleW(None).unwrap().into();
                        ensure_input_box(hinstance);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_PAINT => {
                let state = APP_STATE.lock().unwrap();
                if !state.overlay_enabled {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }

                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                // 더블 버퍼링을 위한 메모리 DC 생성
                let mem_dc = CreateCompatibleDC(hdc);
                let mem_bitmap = CreateCompatibleBitmap(hdc, width, height);
                let old_bitmap = SelectObject(mem_dc, mem_bitmap);

                // 배경 초기화 (검정색 - 투명 처리됨)
                let hbr = CreateSolidBrush(rgb(0, 0, 0));
                let _ = FillRect(mem_dc, &rect, hbr);
                let _ = DeleteObject(hbr);

                // 폰트 설정
                let hfont = CreateFontW(
                    20,
                    0,
                    0,
                    0,
                    FW_BOLD.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    DEFAULT_PITCH.0 as u32,
                    w!("맑은 고딕"),
                );
                let old_font = SelectObject(mem_dc, hfont);
                let _ = SetBkMode(mem_dc, TRANSPARENT);

                let display_text = format!(
                    "Maple Overlay ON (Ctrl + F1)\nNickname: {}\nAPI Key: {}\nResolution: {}x{}",
                    if state.nickname.is_empty() { "None" } else { &state.nickname },
                    if state.api_key.is_empty() { "None" } else { "********" }, // 보안상 가림
                    width,
                    height
                );
                let mut text: Vec<u16> = display_text.encode_utf16().collect();

                let mut text_rect = rect;
                text_rect.top = height / 20;

                // 흰색 테두리 그리기
                let _ = SetTextColor(mem_dc, rgb(255, 255, 255));
                for dx in &[-1, 0, 1] {
                    for dy in &[-1, 0, 1] {
                        if *dx == 0 && *dy == 0 {
                            continue;
                        }
                        let mut temp_rect = text_rect;
                        temp_rect.left += dx;
                        temp_rect.right += dx;
                        temp_rect.top += dy;
                        temp_rect.bottom += dy;
                        let _ = DrawTextW(mem_dc, &mut text, &mut temp_rect, DT_CENTER);
                    }
                }

                // 검정색 메인 글자 그리기
                let _ = SetTextColor(mem_dc, rgb(1, 1, 1));
                let _ = DrawTextW(mem_dc, &mut text, &mut text_rect, DT_CENTER);

                // 메모리 DC의 내용을 실제 화면 DC로 한 번에 복사
                let _ = BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

                // 리소스 해제
                SelectObject(mem_dc, old_font);
                let _ = DeleteObject(hfont);
                SelectObject(mem_dc, old_bitmap);
                let _ = DeleteObject(mem_bitmap);
                let _ = DeleteDC(mem_dc);

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_DESTROY => {
                APP_STATE.lock().unwrap().running = false;
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ... find_maplestory_window 및 enum_windows_proc 로직은 동일 (생략 가능하나 유지됨) ...

// ===============================
// EnumWindows helpers (기존 로직 유지)
// ===============================
struct FindResult {
    hwnd: Option<HWND>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let result = &mut *(lparam.0 as *mut FindResult);
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL::from(true);
    }

    let mut class_buffer = [0u16; 256];
    unsafe {
        let class_len = GetClassNameW(hwnd, &mut class_buffer);
        if String::from_utf16_lossy(&class_buffer[..class_len as usize]) != "MapleStoryClass" {
            return BOOL::from(true);
        }
    }

    unsafe {
        if GetWindow(hwnd, GW_OWNER).0 != 0 {
            return BOOL::from(true);
        }
    }

    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        let mut path = [0u16; 260];
        let mut size = path.len() as u32;
        if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(path.as_mut_ptr()),
            &mut size,
        )
        .is_ok()
        {
            if String::from_utf16_lossy(&path[..size as usize])
                .to_lowercase()
                .ends_with("maplestory.exe")
            {
                result.hwnd = Some(hwnd);
                unsafe {
                let _ = CloseHandle(handle);
                    }
                return BOOL::from(false);
            }
        }
        unsafe {
            let _ = CloseHandle(handle);
        }
    }
    BOOL::from(true)
}

unsafe fn find_maplestory_window() -> Option<HWND> {
    unsafe {
        let mut result = FindResult { hwnd: None };
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut result as *mut _ as isize),
        );
        result.hwnd
    }
}

// ===============================
// main
// ===============================
fn main() -> Result<()> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
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

        let _ = SetLayeredWindowAttributes(overlay_hwnd, rgb(0, 0, 0), 255, LWA_COLORKEY);
        let _ = ShowWindow(overlay_hwnd, SW_SHOW);

        let _ = RegisterHotKey(
            overlay_hwnd,
            HOTKEY_ID_TOGGLE,
            HOT_KEY_MODIFIERS(MOD_CONTROL.0),
            VK_F1.0 as u32,
        );
        let _ = RegisterHotKey(
            overlay_hwnd,
            HOTKEY_ID_NICKNAME,
            HOT_KEY_MODIFIERS(MOD_CONTROL.0),
            VK_F2.0 as u32,
        );

        let mut msg = MSG::default();
        while APP_STATE.lock().unwrap().running {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let (enabled, _) = {
                let state = APP_STATE.lock().unwrap();
                (state.overlay_enabled, state.nickname.is_empty())
            };

            if enabled {
                if let Some(maple_hwnd) = find_maplestory_window() {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(maple_hwnd, &mut rect);

                    if IsWindowVisible(maple_hwnd).as_bool() && !IsIconic(maple_hwnd).as_bool() {
                        let _ = ShowWindow(overlay_hwnd, SW_SHOWNOACTIVATE);
                        let _ = MoveWindow(
                            overlay_hwnd,
                            rect.left,
                            rect.top,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                            true,
                        );
                        let _ = InvalidateRect(overlay_hwnd, None, BOOL::from(true));
                    } else {
                        let _ = ShowWindow(overlay_hwnd, SW_HIDE);
                    }
                } else {
                    let _ = ShowWindow(overlay_hwnd, SW_HIDE);
                }
            }
            Sleep(16);
        }
    }
    Ok(())
}
