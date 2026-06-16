// FrameTimingStats.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QLabel>
#include <QTimer>
#include <QFrame>
#include <QGroupBox>
#include <QTreeWidget>
#include <QTreeWidgetItem>

#include "Qt/main.h"

class FrameTimingDialog_t : public QDialog
{
	Q_OBJECT

public:
	FrameTimingDialog_t(QWidget *parent = 0);
	~FrameTimingDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QTimer *updateTimer;
	QCheckBox *timingEnable;
	QTreeWidgetItem *frameTimeAbs;
	QTreeWidgetItem *frameTimeDel;
	QTreeWidgetItem *frameTimeWork;
	QTreeWidgetItem *frameTimeWorkPct;
	QTreeWidgetItem *frameTimeIdle;
	QTreeWidgetItem *frameTimeIdlePct;
	QTreeWidgetItem *frameLateCount;
	QTreeWidgetItem *videoTimeAbs;
	QGroupBox *statFrame;

	QTreeWidget *tree;
	QPushButton *resetButton;
	QPushButton *closeButton;
	QTreeWidgetItem *headerItem;

private:
	void updateTimingStats(void);

public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void resetTimingClicked(void);
	void timingEnableChanged(int state);
};
