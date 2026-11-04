// folder_watch.h: tells Dreamride when a song folder changes, so the library can rescan without a
// restart. One Win32 change notification per folder (a file added, removed, renamed or written);
// Changed() only polls the handles with a zero timeout, so it costs microseconds a frame and never
// blocks. A folder that cannot be watched (missing, or a share that does not notify) is reported by
// Unwatched(), and the caller rescans it on a period instead (ui::RescanClock::poll). Tested in
// world/tests/ui_logic_test.cpp against a real folder.
#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <string>
#include <vector>

class FolderWatch {
 public:
  FolderWatch() = default;
  explicit FolderWatch(const std::vector<std::string>& dirs) { Watch(dirs); }
  ~FolderWatch() { Close(); }
  FolderWatch(const FolderWatch&) = delete;
  FolderWatch& operator=(const FolderWatch&) = delete;

  // Watches each folder (UTF-8 paths; not their subfolders, as the library does not list them).
  void Watch(const std::vector<std::string>& dirs) {
    Close();
    for (const std::string& d : dirs) {
      int n = MultiByteToWideChar(CP_UTF8, 0, d.c_str(), -1, nullptr, 0);
      std::wstring w(n > 0 ? n : 1, L'\0');
      if (n > 0) MultiByteToWideChar(CP_UTF8, 0, d.c_str(), -1, &w[0], n);
      HANDLE h = FindFirstChangeNotificationW(w.c_str(), FALSE,
          FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE);
      if (h == INVALID_HANDLE_VALUE) unwatched_ = true;
      else handles_.push_back(h);
    }
  }

  // True when any watched folder changed since the last call. Never waits.
  bool Changed() {
    bool any = false;
    for (HANDLE& h : handles_) {
      if (h == INVALID_HANDLE_VALUE) continue;
      if (WaitForSingleObject(h, 0) != WAIT_OBJECT_0) continue;
      any = true;
      if (!FindNextChangeNotification(h)) {  // cannot re-arm: fall back to polling this folder
        FindCloseChangeNotification(h);
        h = INVALID_HANDLE_VALUE;
        unwatched_ = true;
      }
    }
    return any;
  }

  // Whether some folder is not watched (so the library must also poll).
  bool Unwatched() const { return unwatched_; }

 private:
  void Close() {
    for (HANDLE h : handles_) if (h != INVALID_HANDLE_VALUE) FindCloseChangeNotification(h);
    handles_.clear();
    unwatched_ = false;
  }
  std::vector<HANDLE> handles_;
  bool unwatched_ = false;
};
