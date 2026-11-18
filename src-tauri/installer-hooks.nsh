; Remove the elevated scheduled task so an uninstalled Isle leaves nothing behind.
!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'schtasks /Delete /TN "Isle Elevated" /F'
  Sleep 300
!macroend
