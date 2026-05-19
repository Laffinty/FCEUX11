// GamePadConf.h
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
#include <QFrame>
#include <QGroupBox>
#include <QTreeView>
#include <QTreeWidget>
#include <QCloseEvent>

#include "Qt/main.h"

class AboutWindow : public QDialog
{
   Q_OBJECT

	public:
		AboutWindow(QWidget *parent = 0);
		~AboutWindow(void);

		void retranslateUi(void);

	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

	private:
		QLabel *versionLabel;
		QLabel *licenseLabel;
		QLabel *copyrightLabel;
		QLabel *urlLabel;
		QPushButton *viewLicenseButton;
		QPushButton *closeButton;

	public slots:
		void closeWindow(void);
		void openLicense(void);

};
