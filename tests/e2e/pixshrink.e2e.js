describe.skip('PixShrink wizard flow', () => {
  it('shows the window from the tray icon');
  it('hides instead of quitting on window close');
  it('accepts drag and drop on page 1');
  it('loads files from the OPEN button');
  it('enables NEXT after format selection');
  it('skips quality when PNG is selected');
  it('shows quality page for JPEG');
  it('keeps aspect ratio locked when width changes');
  it('leaves height unchanged when aspect lock is off');
  it('keeps pixel and percent inputs synchronized');
  it('updates estimated size within the debounce window');
  it('refreshes rename preview instantly');
  it('writes converted files and hides the window after save');
  it('resets the wizard after the window is restored');
});
