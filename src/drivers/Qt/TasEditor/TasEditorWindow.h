// TasEditorWindow.h
//

#pragma once

#include <stdint.h>
#include <time.h>
#include <string>
#include <list>
#include <map>

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QSplitter>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QRadioButton>
#include <QLineEdit>
#include <QLabel>
#include <QFrame>
#include <QGroupBox>
#include <QTreeView>
#include <QTreeWidget>
#include <QTreeWidgetItem>
#include <QCloseEvent>
#include <QScrollBar>
#include <QScrollArea>
#include <QMenu>
#include <QMenuBar>
#include <QAction>
#include <QFont>
#include <QPainter>
#include <QShortcut>
#include <QTabWidget>
#include <QProgressBar>
#include <QStackedWidget>
#include <QClipboard>

#include "Qt/config.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/TasEditor/taseditor_config.h"
#include "Qt/TasEditor/taseditor_project.h"
#include "Qt/TasEditor/greenzone.h"
#include "Qt/TasEditor/selection.h"
#include "Qt/TasEditor/markers_manager.h"
#include "Qt/TasEditor/snapshot.h"
#include "Qt/TasEditor/bookmarks.h"
#include "Qt/TasEditor/branches.h"
#include "Qt/TasEditor/history.h"
#include "Qt/TasEditor/playback.h"
#include "Qt/TasEditor/recorder.h"
#include "Qt/TasEditor/taseditor_lua.h"
#include "Qt/TasEditor/splicer.h"
#include "Qt/TasEditor/TasEditorTimeline.h"
#include "Qt/TasEditor/bookmarkPreviewPopup.h"
#include "Qt/TasEditor/markerDragPopup.h"
#include "Qt/TasEditor/TasFindNoteWindow.h"
//#include "Qt/TasEditor/editor.h"
//#include "Qt/TasEditor/popup_display.h"


class TasEditorWindow;

struct NewProjectParameters
{
	int inputType;
	bool copyCurrentInput;
	bool copyCurrentMarkers;
	std::wstring authorName;
};

class TasEditorWindow;  // forward declaration for QPianoRoll

class TasRecentProjectAction : public QAction
{
	Q_OBJECT

	public:
		TasRecentProjectAction( QString title, QWidget *parent = 0);
		~TasRecentProjectAction(void);

		std::string  path;

	public slots:
		void activateCB(void);

};

class TasEditorSplitter : public QSplitter
{
	Q_OBJECT

	public:
		TasEditorSplitter(QWidget *parent = 0);
		~TasEditorSplitter(void);

	protected:
		void resizeEvent(QResizeEvent *event);

		bool panelInitDone;
};

class TasEditorWindow : public QDialog
{
	Q_OBJECT

	public:
		TasEditorWindow(QWidget *parent = 0);
		~TasEditorWindow(void);

		void retranslateUi(void);

		QPianoRoll  *pianoRoll;

		TASEDITOR_PROJECT project;
		TASEDITOR_CONFIG taseditorConfig;
		TASEDITOR_LUA taseditor_lua;
		MARKERS_MANAGER markersManager;
		BOOKMARKS bookmarks;
		//PIANO_ROLL pianoRoll;
		SPLICER splicer;
		//EDITOR editor;
		GREENZONE greenzone;
		SELECTION selection;
		PLAYBACK  playback;
		RECORDER  recorder;
		HISTORY   history;
		BRANCHES  branches;

		void initHotKeys(void);
		void updateCaption(void);
		bool loadProject(const char* fullname);
		void importMovieFile( const char *path );
		void loadClipboard(const char *txt);
		void toggleInput(int start, int end, int joy, int button, int consecutivenessTag);
		void setInputUsingPattern(int start, int end, int joy, int button, int consecutivenessTag);
		bool handleColumnSet(void);
		bool handleColumnSetUsingPattern(void);
		bool handleInputColumnSet(int joy, int button);
		bool handleInputColumnSetUsingPattern(int joy, int button);
		bool updateHistoryItems(void);

		int  requestWindowClose(void);

		QPoint getPreviewPopupCoordinates(void);

	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

		QMenuBar  *buildMenuBar(void);
		void buildPianoRollDisplay(void);
		void buildSideControlPanel(void);
		void initPatterns(void);

		void retranslateMenuBar(void);

		QMenu     *recentProjectMenu;
		QAction   *followUndoAct;
		QAction   *followMkrAct;
		QAction   *enaHotChgAct;
		QAction   *dpyBrnchDescAct;
		QAction   *dpyBrnchScrnAct;
		QAction   *enaGrnznAct;
		QAction   *afPtrnSkipLagAct;
		QAction   *adjInputLagAct;
		QAction   *drawInputDragAct;
		QAction   *cmbRecDrawAct;
		QAction   *use1PforRecAct;
		QAction   *useInputColSetAct;
		QAction   *bindMkrInputAct;
		QAction   *emptyNewMkrNotesAct;
		QAction   *oldCtlBrnhSchemeAct;
		QAction   *brnchRestoreMovieAct;
		QAction   *hudInScrnBranchAct;
		QAction   *pauseAtEndAct;
		QAction   *showToolTipsAct;
		QAction   *autoLuaAct;

		TasEditorSplitter  *mainHBox;
		QFrame     *pianoRollFrame;
		QWidget    *pianoRollContainerWidget;
		QWidget    *controlPanelContainerWidget;
		QScrollBar *pianoRollHBar;
		QScrollBar *pianoRollVBar;
		QPushButton     *upperMarkerLabel;
		QPushButton     *lowerMarkerLabel;
		UpperMarkerNoteEdit  *upperMarkerNote;
		LowerMarkerNoteEdit  *lowerMarkerNote;
		QTabWidget *bkmkBrnchStack;

		QVBoxLayout *ctlPanelMainVbox;
		QGroupBox  *playbackGBox;
		QGroupBox  *recorderGBox;
		QGroupBox  *splicerGBox;
		//QGroupBox  *luaGBox;
		//QGroupBox  *historyGBox;
		QFrame     *bbFrame;

		QPushButton  *rewindMkrBtn;
		QPushButton  *rewindFrmBtn;
		QPushButton  *playPauseBtn;
		QPushButton  *advFrmBtn;
		QPushButton  *advMkrBtn;
		QProgressBar *progBar;
		QCheckBox    *followCursorCbox;
		QCheckBox    *turboSeekCbox;
		QCheckBox    *autoRestoreCbox;

		QCheckBox    *recRecordingCbox;
		QCheckBox    *recSuperImposeCbox;
		QCheckBox    *recUsePatternCbox;
		QRadioButton *recAllBtn;
		QRadioButton *rec1PBtn;
		QRadioButton *rec2PBtn;
		QRadioButton *rec3PBtn;
		QRadioButton *rec4PBtn;

		QLabel      *selectionLbl;
		QLabel      *clipboardLbl;

		//QPushButton *runLuaBtn;
		//QCheckBox   *autoLuaCBox;

		QTreeWidget *histTree;

		QPushButton *prevMkrBtn;
		QPushButton *nextMkrBtn;
		QPushButton *similarBtn;
		QPushButton *moreBtn;

		QClipboard *clipboard;

		QShortcut  *hotkeyShortcut[HK_MAX];

		std::vector<std::string> patternsNames;
		std::vector<std::vector<uint8_t>> patterns;
		std::list <std::string*> projList;

		bool mustCallManualLuaFunction;
		bool recentProjectMenuReset;
	private:

		int initModules(void);
		bool saveProject(bool save_compact = false);
		bool saveProjectAs(bool save_compact = false);
		bool askToSaveProject(void);
		bool saveCompactGetFilename( QString &filepath );
		void updateToolTips(void);

		void clearProjectList(void);
		void buildRecentProjectMenu(void);
		void saveRecentProjectMenu(void);
		void addRecentProject(const char *prog);


	public slots:
		void closeWindow(void);
		void frameUpdate(void);
		void updateCheckedItems(void);
		void updateRecordStatus(void);
	private slots:
		void openProject(void);
		void saveProjectCb(void);
		void saveProjectAsCb(void);
		void saveProjectCompactCb(void);
		void createNewProject(void);
		void importMovieFile(void);
		void exportMovieFile(void);
		void openOnlineDocs(void);
		void recordingChanged(int);
		void recordInputChanged(int);
		void superImposedChanged(int);
		void usePatternChanged(int);
		void playbackPauseCB(void);
		void playbackFrameRewind(void);
		void playbackFrameForward(void);
		void playbackFrameRewindFull(void);
		void playbackFrameForwardFull(void);
		void scrollSelectionUpOne(void);
		void scrollSelectionDnOne(void);
		void editUndoCB(void);
		void editRedoCB(void);
		void editUndoSelCB(void);
		void editRedoSelCB(void);
		void editDeselectAll(void);
		void editSelectAll(void);
		void editSelBtwMkrs(void);
		void editReselectClipboard(void);
		void editCutCB(void);
		void editCopyCB(void);
		void editPasteCB(void);
		void editPasteInsertCB(void);
		void editClearCB(void);
		void editDeleteCB(void);
		void editCloneCB(void);
		void editInsertCB(void);
		void editInsertNumFramesCB(void);
		void editTruncateMovieCB(void);
		void openFindNoteWindow(void);
		void dpyBrnchScrnChanged(bool);
		void dpyBrnchDescChanged(bool);
		void enaHotChgChanged(bool);
		void followUndoActChanged(bool);
		void followMkrActChanged(bool);
		void enaGrnznActChanged(bool);
		void afPtrnSkipLagActChanged(bool);
		void adjInputLagActChanged(bool);
		void drawInputDragActChanged(bool);
		void cmbRecDrawActChanged(bool);
		void use1PforRecActChanged(bool);
		void useInputColSetActChanged(bool);
		void bindMkrInputActChanged(bool);
		void emptyNewMkrNotesActChanged(bool);
		void oldCtlBrnhSchemeActChanged(bool);
		void brnchRestoreMovieActChanged(bool);
		void hudInScrnBranchActChanged(bool);
		void pauseAtEndActChanged(bool);
		void showToolTipsActChanged(bool);
		void upperMarkerLabelClicked(void);
		void lowerMarkerLabelClicked(void);
		void histTreeItemActivated(QTreeWidgetItem*,int);
		void playbackFollowCursorCb(bool);
		void playbackAutoRestoreCb(bool);
		void playbackTurboSeekCb(bool);
		void openProjectSaveOptions(void);
		void setGreenzoneCapacity(void);
		void setMaxUndoCapacity(void);
		void setCurrentPattern(int);
		void tabViewChanged(int);
		void findSimilarNote(void);
		void findNextSimilarNote(void);
		void jumpToPreviousMarker(void);
		void jumpToNextMarker(void);
		void openAboutWindow(void);
		void autoLuaRunChanged(bool);
		void manLuaRun(void);
		void setMarkers(void);
		void removeMarkers(void);
		void ungreenzoneSelectedFrames(void);
		void activateHotkey( int hkIdx, QShortcut *shortcut );
		void changePianoRollFontCB(void);
		void changeBookmarksFontCB(void);
		void changeBranchesFontCB(void);

	friend class RECORDER;
	friend class SPLICER;
	friend class SELECTION;
	friend class PLAYBACK;
	friend class HISTORY;
	friend class MARKERS_MANAGER;
	friend class TASEDITOR_PROJECT;
	friend class QPianoRoll;
};

extern TASEDITOR_PROJECT *project;
extern TASEDITOR_CONFIG *taseditorConfig;
extern TASEDITOR_LUA *taseditor_lua;
extern MARKERS_MANAGER *markersManager;
extern BOOKMARKS *bookmarks;
extern GREENZONE *greenzone;
extern PLAYBACK *playback;
extern RECORDER *recorder;
extern SPLICER *splicer;
extern HISTORY *history;
extern SELECTION *selection;
extern BRANCHES *branches;

bool tasWindowIsOpen(void);

void tasWindowSetFocus(bool val);

bool isTaseditorRecording(void);
void recordInputByTaseditor(void);

uint64_t getTasEditorTime(void);

extern TasEditorWindow *tasWin;
