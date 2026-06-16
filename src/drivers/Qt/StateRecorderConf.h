// StateRecorderConf.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QSpinBox>
#include <QComboBox>
#include <QPushButton>
#include <QRadioButton>
#include <QProgressBar>
#include <QLineEdit>
#include <QLabel>
#include <QFrame>
#include <QGroupBox>
#include <QTimer>

struct StateRecorderConfigData;

class StateRecorderDialog_t : public QDialog
{
	Q_OBJECT

public:
	StateRecorderDialog_t(QWidget *parent = 0);
	~StateRecorderDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QSpinBox     *snapMinutes;
	QSpinBox     *snapSeconds;
	QSpinBox     *snapFrames;
	QSpinBox     *historyDuration;
	QSpinBox     *pauseDuration;
	QCheckBox    *recorderEnable;
	QLineEdit    *numSnapsLbl;
	QLineEdit    *snapMemSizeLbl;
	QLineEdit    *totalMemUsageLbl;
	QLineEdit    *saveTimeLbl;
	QPushButton  *applyButton;
	QPushButton  *closeButton;
	QComboBox    *cmprLvlCbox;
	QComboBox    *pauseOnLoadCbox;
	QGroupBox    *snapTimeGroup;
	QGroupBox    *snapFramesGroup;
	QRadioButton *snapFrameSelBtn;
	QRadioButton *snapTimeSelBtn;
	QLineEdit    *recStatusLbl;
	QLineEdit    *recBufSizeLbl;
	QPushButton  *startStopButton;
	QProgressBar *bufUsage;
	QTimer       *updateTimer;

	QLabel       *historyMinutesLbl;
	QLabel       *frameRangeLbl;
	QLabel       *timeMinutesLbl;
	QLabel       *timeSecondsLbl;
	QLabel       *pauseDurationLbl;
	QLabel       *numSnapsHeaderLbl;
	QLabel       *snapSizeHeaderLbl;
	QLabel       *totalSizeHeaderLbl;
	QLabel       *stateHeaderLbl;
	QLabel       *bufSizeHeaderLbl;
	QLabel       *snapshotSaveTimeLbl;
	QLabel       *stateRecorderTitleLbl;
	QGroupBox    *retainHistoryFrame;
	QGroupBox    *compressionFrame;
	QGroupBox    *snapshotTimingFrame;
	QGroupBox    *pauseOnLoadFrame;
	QGroupBox    *durationFrame;
	QGroupBox    *memUsageFrame;
	QGroupBox    *cpuUsageFrame;
	QGroupBox    *recorderStatusFrame;
	QGroupBox    *bufferUseFrame;

	double       saveTimeMs;
	bool         snapUseTime;

	bool dataSavedCheck(void);
	void recalcMemoryUsage(void);
	void updateStartStopBuffon(void);
	void updateRecorderStatusLabel(void);
	void updateBufferSizeStatus(void);
	void updateStatusDisplay(void);
	void packConfig( StateRecorderConfigData &config );

public slots:
	void closeWindow(void);
private slots:
	void applyChanges(void);
	void updatePeriodic(void);
	void startStopClicked(void);
	void snapTimeModeClicked(void);
	void snapFrameModeClicked(void);
	void spinBoxValueChanged(int newValue);
	void enableChanged(int);
	void compressionLevelChanged(int newValue);
	void pauseOnLoadChanged(int index);
};
