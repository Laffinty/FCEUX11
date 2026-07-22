// MsgLogViewer.h
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
#include <QPlainTextEdit>

#include "Qt/main.h"

class MsgLogViewDialog_t : public QDialog
{
	Q_OBJECT

public:
	MsgLogViewDialog_t(QWidget *parent = 0);
	~MsgLogViewDialog_t(void);

	// hotfix4 D-15: handle LanguageChange so tr() in the constructor
	// re-fires when the user switches language.
	void changeEvent(QEvent *event) override;

protected:
	void closeEvent(QCloseEvent *event);

	QTimer *updateTimer;
	QPlainTextEdit *txtView;
	// hotfix4 D-15: promoted from constructor locals so changeEvent can
	// re-apply tr() on language switch.
	QPushButton *clearBtn;
	QPushButton *closeBtn;

	size_t totalLines;

private:
public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void clearLog(void);
};
