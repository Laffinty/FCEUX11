
//

#ifndef __GameAppH__
#define __GameAppH__

#include <vector>
#include <string>

#include <QApplication>
#include <QMainWindow>
#include <QWidget>
#include <QPushButton>
#include <QMenu>
#include <QMenuBar>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QKeyEvent>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QTimer>
#include <QThread>
#include <QCursor>
#include <QMutex>
#include <QColor>
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
#include <QRecursiveMutex>
#endif

#include "Qt/ColorMenu.h"
#include "Qt/MenuCatalog.h"
#include "Qt/ConsoleViewerGL.h"
#include "Qt/ConsoleViewerSDL.h"
#include "Qt/ConsoleViewerQWidget.h"
#include "Qt/GamePadConf.h"
#include "Qt/AviRecord.h"

#ifdef _WIN32
// v0.3.15.x PHASE-3: ITaskbarList3 wrapper for Snap Layouts.
#include "platform/win11/TaskbarProgress.h"
#endif

class  emulatorThread_t : public QThread
{
	Q_OBJECT

	protected:
		void run( void ) override;

	public:
		emulatorThread_t( QObject *parent = 0 );

		void setPriority( QThread::Priority priority );

		
		void signalFrameFinished(void);
		void signalRomLoad(const char *rom);
	private:
		void init(void);

		

	signals:
		void finished(void);
		void frameFinished(void);
		void loadRomRequest( QString s );
};

class  consoleMenuBar : public QMenuBar
{
	public:
		consoleMenuBar(QWidget *parent = 0);
		~consoleMenuBar(void);

	protected:
		void keyPressEvent(QKeyEvent *event);
		void keyReleaseEvent(QKeyEvent *event);
};

class  autoFireMenuAction : public QAction
{
	Q_OBJECT

	public:
		autoFireMenuAction(int on, int off, QString name, QWidget *parent = 0);
		~autoFireMenuAction(void);

		bool isMatch( int on, int off );

		void setPattern( int on, int off );

		int  getOnValue(void){ return onFrames; };
		int  getOffValue(void){ return offFrames; };

	public slots:
		void activateCB(void);

	private:
		int  onFrames;
		int  offFrames;
};

class  consoleRecentRomAction : public QAction
{
	Q_OBJECT

	public:
		consoleRecentRomAction( QString title, QWidget *parent = 0);
		~consoleRecentRomAction(void);

		std::string  path;

	public slots:
		void activateCB(void);

};

class  consoleWin_t : public QMainWindow
{
	Q_OBJECT

	public:
		consoleWin_t(QWidget *parent = 0);
		~consoleWin_t(void);

		ConsoleViewGL_t       *viewport_GL;
		ConsoleViewSDL_t      *viewport_SDL;
		ConsoleViewQWidget_t  *viewport_QWidget;
		ConsoleViewerBase     *viewport_Interface;

		void setCyclePeriodms( int ms );

#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
		QRecursiveMutex *mutex;
#else
		QMutex *mutex;
#endif

		int  videoInit(void);
		void videoReset(void);
		void requestClose(void);

	 	void QueueErrorMsgWindow( const char *msg );

		int  showListSelectDialog( const char *title, std::vector <std::string> &l );

		

		int loadVideoDriver( int driverId, bool force = false );
		int unloadVideoDriver(void);

		double getRefreshRate(void){ return refreshRate; }

		emulatorThread_t *emulatorThread;
		AviRecordDiskThread_t *aviDiskThread;

		void addRecentRom( const char *rom );

		QSize  calcRequiredSize(void);

		void setViewportAspect(void);

		void loadCursor(void);
		void setViewerCursor( QCursor s );
		void setViewerCursor( Qt::CursorShape s );
		Qt::CursorShape getViewerCursor(void);

		void setMenuAccessPauseEnable(bool enable);
		void setContextMenuEnable(bool enable);
		void setSoundUseGlobalFocus(bool enable);

		void OpenHelpWindow(std::string subpage = "");

		int  getPeriodicInterval(void);

		QColor *getVideoBgColorPtr(void){ return &videoBgColor; }

	protected:
		consoleMenuBar *menubar;

		QMenu *fileMenu;
		QMenu *optMenu;
		QMenu *emuMenu;
		QMenu *helpMenu;
		QMenu *recentRomMenu;
		QMenu *languageMenu;
		// v0.3.15 PR-A: 5+1 audience-tiered menu model.
		// Tools / Debug / Movie are collected under one "Advanced" top-level
		// menu with sub-menus. Hidden by SDL.HideAdvancedMenu = ON.
		QMenu *advMenu;
		QMenu *advEmuMenu;     // soft reset / GG / FKB / FDS / VS / RAM init / AutoFire
		QMenu *advMovieMenu;   // full movie + AVI/WAV record
		QMenu *advDebugMenu;   // debugger / hex / ppu / sprite / nt / trace / cdl / gg encode / iNes
		QMenu *advMemoryMenu;  // cheats / RAM search / RAM watch
		QMenu *advMiscMenu;    // frame timing / palette editor / avi riff / tas editor
		QMenu *advSettingsMenu; // input / gamepad / hotkey / palette / timing / stateRec / movieOpt / autoResume

		QActionGroup *languageActionGroup;
		
		QAction *openROM = nullptr;
		QAction *closeROM = nullptr;
		QAction *playNSF = nullptr;
		QAction *loadStateAct = nullptr;
		QAction *saveStateAct = nullptr;
		QAction *quickLoadAct = nullptr;
		QAction *quickSaveAct = nullptr;
		QAction *loadLuaAct = nullptr;
		QAction *scrShotAct = nullptr;
		QAction *quitAct = nullptr;
		QAction *inputConfig = nullptr;
		QAction *gamePadConfig = nullptr;
		QAction *gameSoundConfig = nullptr;
		QAction *gameVideoConfig = nullptr;
		QAction *hotkeyConfig = nullptr;
		QAction *paletteConfig = nullptr;
		QAction *guiConfig = nullptr;
		QAction *stateRecordConfig = nullptr;
		QAction *timingConfig = nullptr;
		QAction *movieConfig = nullptr;
		QAction *autoResume = nullptr;
		QAction *winSizeAct[4] = {};
		QAction *fullscreen = nullptr;
		QAction *aboutAct = nullptr;
		QAction *aboutActQt = nullptr;
		QAction *msgLogAct = nullptr;
		QAction *state[10] = {};
		QAction *powerAct = nullptr;
		QAction *resetAct = nullptr;
		QAction *sresetAct = nullptr;
		QAction *pauseAct = nullptr;
		QAction *gameGenieAct = nullptr;
		QAction *loadGgROMAct = nullptr;
		QAction *insCoinAct = nullptr;
		QAction *fdsSwitchAct = nullptr;
		QAction *fdsEjectAct = nullptr;
		QAction *fdsLoadBiosAct = nullptr;
		QAction *cheatsAct = nullptr;
		QAction *ramWatchAct = nullptr;
		QAction *ramSearchAct = nullptr;
		QAction *debuggerAct = nullptr;
		QAction *codeDataLogAct = nullptr;
		QAction *traceLogAct = nullptr;
		QAction *hexEditAct = nullptr;
		QAction *ppuViewAct = nullptr;
		QAction *oamViewAct = nullptr;
		QAction *ntViewAct = nullptr;
		QAction *ggEncodeAct = nullptr;
		QAction *iNesEditAct = nullptr;
		QAction *openMovAct = nullptr;
		QAction *playMovBeginAct = nullptr;
		QAction *stopMovAct = nullptr;
		QAction *recMovAct = nullptr;
		QAction *region[3] = {};
		QAction *ramInit[4] = {};
		QAction *recAviAct = nullptr;
		QAction *recAsAviAct = nullptr;
		QAction *stopAviAct = nullptr;
		QAction *recWavAct = nullptr;
		QAction *recAsWavAct = nullptr;
		QAction *stopWavAct = nullptr;
		QAction *tasEditorAct = nullptr;
		//QAction *aviHudAct;
		//QAction *aviMsgAct;

		QTimer  *gameTimer;
		QColor   videoBgColor;
		ColorMenuItem *bgColorMenuItem;

		std::string errorMsg;
		bool        errorMsgValid;
		bool        closeRequested;
		bool        recentRomMenuReset;
		bool        firstResize;
		bool        mainMenuEmuPauseSet;
		bool        mainMenuEmuWasPaused;
		bool        mainMenuPauseWhenActv;
		bool        scrHandlerConnected;
		bool        contextMenuEnable;
		bool        soundUseGlobalFocus;
		bool        autoHideMenuFullscreen;
		// v0.3.15 PR-A: when true, hide the "Advanced" top-level menu
		// and collapse back to 4 top-level menus (File / Emulation / Options / Help).
		bool        hideAdvancedMenu;

		std::list <std::string*> romList;
		std::vector <autoFireMenuAction*> afActList;
		autoFireMenuAction *afActCustom;

		double       refreshRate;
		unsigned int updateCounter;
#ifdef WIN32
		HWND   helpWin;
		// v0.3.15.x PHASE-3: ITaskbarList3 wrapper for Snap Layouts
		// progress + overlay icon. Owned by consoleWin_t; init() is
		// called from the constructor once the QMainWindow has a
		// stable HWND, and release() runs in the destructor.
		fceu11::platform::win11::TaskbarProgress *taskbarProgress;
#else
		int    helpWin;
#endif
	protected:
		void resizeEvent(QResizeEvent *event) override;
		void closeEvent(QCloseEvent *event) override;
		void changeEvent(QEvent *event) override;
		void keyPressEvent(QKeyEvent *event) override;
		void keyReleaseEvent(QKeyEvent *event) override;
		void dragEnterEvent(QDragEnterEvent *event) override;
		void dropEvent(QDropEvent *event) override;
		void showEvent(QShowEvent *event) override;
		void contextMenuEvent(QContextMenuEvent *event) override;
		void syncActionConfig( QAction *act, const char *property );
		void showErrorMsgWindow(void);

	private:
		void initHotKeys(void);
		void initScreenHandler(void);
		void createMainMenu(void);
		void buildRecentRomMenu(void);
		void saveRecentRomMenu(void);
		void clearRomList(void);
		void setRegion(int region);
		void changeState(int slot);
		void saveState(int slot);
		void loadState(int slot);
		void transferVideoBuffer(void);
		void syncAutoFirePatternMenu(void);

		std::string findHelpFile(void);

	public slots:
		void openDebugWindow(void);
		void openHexEditor(void);
		void openGamePadConfWin(void);
		void toggleFullscreen(void);
		void toggleMenuVis(void);
		void recordMovie(void);
		void winResizeIx(int iScale);
		void loadTranslation(const QString &langCode);
		void retranslateUi(void);
	private slots:
		void closeApp(void);
		void openROMFile(void);
		void loadNSF(void);
		void loadStateFrom(void);
		void saveStateAs(void);
		void quickLoad(void);
		void quickSave(void);
		void closeROMCB(void);
		void aboutFCEUX(void);
		void aboutQt(void);
		void openOnlineDocs(void);
		void openOfflineDocs(void);
		void openTasEditor(void);
		void openMsgLogWin(void);
		void openInputConfWin(void);
		void openGameSndConfWin(void);
		void openGameVideoConfWin(void);
		void openHotkeyConfWin(void);
		void openPaletteConfWin(void);
		void openGuiConfWin(void);
		void openTimingConfWin(void);
		void openStateRecorderConfWin(void);
		void openPaletteEditorWin(void);
		void openAviRiffViewer(void);
		void openTimingStatWin(void);
		void openMovieOptWin(void);
		void openCodeDataLogger(void);
		void openTraceLogger(void);
		void openFamilyKeyboard(void);
		void toggleAutoResume(void);
		void updatePeriodic(void);
		void changeState0(void);
		void changeState1(void);
		void changeState2(void);
		void changeState3(void);
		void changeState4(void);
		void changeState5(void);
		void changeState6(void);
		void changeState7(void);
		void changeState8(void);
		void changeState9(void);
		void incrementState(void);
		void decrementState(void);
		void loadLua(void);
		void takeScreenShot(void);
		void prepareScreenShot(void);
		void powerConsoleCB(void);
		void consoleHardReset(void);
		void consoleSoftReset(void);
		void consolePause(void);

#ifdef _WIN32
		// v0.3.15.x PHASE-3: drive the Windows taskbar progress bar
		// from long-running operations. pct in [0.0, 1.0]; pct < 0
		// clears the bar. Safe to call from any thread; internally
		// no-op when the ITaskbarList3 wrapper is not bound.
		void setTaskbarProgress(double pct);
		// Drive the progress state (TBPF_* constants from shobjidl.h).
		void setTaskbarState(int tbpfState);
#endif
		void toggleGameGenie(bool checked);
		void loadGameGenieROM(void);
		void loadMostRecentROM(void);
		void setRegionNTSC(void);
		void setRegionPAL(void);
		void setRegionDendy(void);
		void setRamInit0(void);
		void setRamInit1(void);
		void setRamInit2(void);
		void setRamInit3(void);
		void insertCoin(void);
		void fdsSwitchDisk(void);
		void fdsEjectDisk(void);
		void fdsLoadBiosFile(void);
		void emuSpeedUp(void);
		void emuSlowDown(void);
		void emuSlowestSpd(void);
		void emuNormalSpd(void);
		void emuFastestSpd(void);
		void emuCustomSpd(void);
		void emuSetFrameAdvDelay(void);
		void openPPUViewer(void);
		void openOAMViewer(void);
		void openNTViewer(void);
		void openGGEncoder(void);
		void openNesHeaderEditor(void);
		void openCheats(void);
		void openRamWatch(void);
		void openRamSearch(void);
		void openMovie(void);
		void stopMovie(void);
		void playMovieFromBeginning(void);
		void setCustomAutoFire(void);
		void muteSoundVolume(void);
		void incrSoundVolume(void);
		void decrSoundVolume(void);
		void toggleLagCounterDisplay(void);
		void toggleFrameAdvLagSkip(void);
		void toggleMovieBindSaveState(void);
		void toggleMovieFrameDisplay(void);
		void toggleMovieReadWrite(void);
		void toggleInputDisplay(void);
		void toggleTurboMode(void);
		void toggleBackground(void);
		void toggleForeground(void);
		void toggleFamKeyBrdEnable(void);
		void toggleGlobalCheatEnable(void);
		void saveState0(void);
		void saveState1(void);
		void saveState2(void);
		void saveState3(void);
		void saveState4(void);
		void saveState5(void);
		void saveState6(void);
		void saveState7(void);
		void saveState8(void);
		void saveState9(void);
		void loadState0(void);
		void loadState1(void);
		void loadState2(void);
		void loadState3(void);
		void loadState4(void);
		void loadState5(void);
		void loadState6(void);
		void loadState7(void);
		void loadState8(void);
		void loadState9(void);
		void loadPrevState(void);
		void loadNextState(void);
		void mainMenuOpen(void);
		void mainMenuClose(void);
		void warnAmbiguousShortcut( QShortcut*);
		void aviRecordStart(void);
		void aviRecordAsStart(void);
		void aviRecordStop(void);
		void aviAudioEnableChange(bool);
		void aviVideoFormatChanged(int idx);
		void setAviHudEnable(bool);
		void setAviMsgEnable(bool);
		void wavRecordStart(void);
		void wavRecordAsStart(void);
		void wavRecordStop(void);
		void winScreenChanged( QScreen *scr );
		void winActiveChanged(void);
		void emuFrameFinish(void);
		void toggleMenuAutoHide(bool);
		void toggleUseBgPaletteForVideo(bool);
		void videoBgColorChanged( QColor &c );
		void loadRomRequestCB( QString s );
		void videoDriverDestroyed( QObject *obj );

};

extern consoleWin_t *consoleWindow;

#endif
