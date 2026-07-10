// TasFindNoteWindow.h
//

#pragma once

#include <QDialog>
#include <QLineEdit>
#include <QCheckBox>
#include <QRadioButton>
#include <QPushButton>
#include <QGroupBox>
#include <QCloseEvent>
#include <QEvent>

class TasFindNoteWindow : public QDialog
{
	Q_OBJECT

	public:
		TasFindNoteWindow(QWidget *parent = 0);
		~TasFindNoteWindow(void);

		void retranslateUi(void);

	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

		QLineEdit     *searchPattern;
		QCheckBox     *matchCase;
		QRadioButton  *up;
		QRadioButton  *down;
		QPushButton   *nextBtn;
		QPushButton   *closeBtn;
		QGroupBox     *dirGbox;

	public slots:
		void closeWindow(void);

	private slots:
		void matchCaseChanged(bool);
		void upDirectionSelected(void);
		void downDirectionSelected(void);
		void findNextClicked(void);
		void searchPatternChanged(const QString &);
};
