// TasFindNoteWindow.cpp
//

#include <QSettings>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QMessageBox>
#include <QFileInfo>

#include "Qt/TasEditor/TasFindNoteWindow.h"
#include "Qt/TasEditor/TasEditorWindow.h"
#include "Qt/TasEditor/markers_manager.h"

extern TasFindNoteWindow *findWin;

TasFindNoteWindow::TasFindNoteWindow( QWidget *parent )
	: QDialog( parent, Qt::Window )
{
	QSettings  settings;
	QVBoxLayout *mainLayout, *vbox;
	QHBoxLayout *hbox, *hbox1;

	setWindowTitle( tr("Find Note") );

	mainLayout = new QVBoxLayout();
	hbox1      = new QHBoxLayout();
	hbox       = new QHBoxLayout();
	vbox       = new QVBoxLayout();

	setLayout( mainLayout );

	searchPattern = new QLineEdit();
	matchCase     = new QCheckBox( this );
	matchCase->setText( tr("Match Case") );
	up            = new QRadioButton( this );
	up->setText( tr("Up") );
	down          = new QRadioButton( this );
	down->setText( tr("Down") );
	nextBtn       = new QPushButton( this );
	nextBtn->setText( tr("Next") );
	closeBtn      = new QPushButton( this );
	closeBtn->setText( tr("Close") );
	dirGbox       = new QGroupBox( this );
	dirGbox->setTitle( tr("Direction") );

	mainLayout->addWidget( searchPattern );
	mainLayout->addLayout( hbox1 );

	hbox1->addWidget( matchCase );
	hbox1->addWidget( dirGbox );
	hbox1->addLayout( vbox );

	dirGbox->setLayout( hbox );

	hbox->addWidget( up );
	hbox->addWidget( down );

	vbox->addWidget( nextBtn );
	vbox->addWidget( closeBtn );

	findWin = this;

	nextBtn->setDefault(true);

	matchCase->setChecked( taseditorConfig->findnoteMatchCase );
	up->setChecked( taseditorConfig->findnoteSearchUp );
	down->setChecked( !taseditorConfig->findnoteSearchUp );

	searchPattern->setText( QString(markersManager->findNoteString) );

	nextBtn->setEnabled( searchPattern->text().size() > 0 );

	connect( matchCase, SIGNAL(clicked(bool)), this, SLOT(matchCaseChanged(bool)) );
	connect( up       , SIGNAL(clicked(void)), this, SLOT(upDirectionSelected(void)) );
	connect( down     , SIGNAL(clicked(void)), this, SLOT(downDirectionSelected(void)) );
	connect( closeBtn , SIGNAL(clicked(void)), this, SLOT(closeWindow(void)) );
	connect( nextBtn  , SIGNAL(clicked(void)), this, SLOT(findNextClicked(void)) );

	connect( searchPattern, SIGNAL(textChanged(const QString &)), this, SLOT(searchPatternChanged(const QString &)) );

	// Restore Window Geometry
	restoreGeometry(settings.value("tasEditorFindDialog/geometry").toByteArray());
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::retranslateUi(void)
{
	setWindowTitle( tr("Find Note") );
	if (matchCase) matchCase->setText( tr("Match Case") );
	if (up       ) up->setText( tr("Up") );
	if (down     ) down->setText( tr("Down") );
	if (nextBtn  ) nextBtn->setText( tr("Next") );
	if (closeBtn ) closeBtn->setText( tr("Close") );
	if (dirGbox  ) dirGbox->setTitle( tr("Direction") );
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		setWindowTitle( tr("Find Note") );
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
TasFindNoteWindow::~TasFindNoteWindow(void)
{
	QSettings  settings;

	if ( findWin == this )
	{
		findWin = NULL;
	}

	// Save Window Geometry
	settings.setValue("tasEditorFindDialog/geometry", saveGeometry());
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::closeEvent(QCloseEvent *event)
{
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::closeWindow(void)
{
	done(0);
	deleteLater();
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::matchCaseChanged(bool val)
{
	taseditorConfig->findnoteMatchCase = val;
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::upDirectionSelected(void)
{
	taseditorConfig->findnoteSearchUp = true;
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::downDirectionSelected(void)
{
	taseditorConfig->findnoteSearchUp = false;
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::searchPatternChanged(const QString &s)
{
	nextBtn->setEnabled( s.size() > 0 );
}
//----------------------------------------------------------------------------
void TasFindNoteWindow::findNextClicked(void)
{

	if ( searchPattern->text().size() == 0 )
	{
		return;
	}
	strncpy( markersManager->findNoteString, searchPattern->text().toStdString().c_str(), MAX_NOTE_LEN-1 );
	markersManager->findNoteString[MAX_NOTE_LEN-1] = 0;

	// scan frames from current Selection to the border
	int cur_marker = 0;
	bool result;
	int movie_size = currMovieData.getNumRecords();
	int current_frame = selection->getCurrentRowsSelectionBeginning();
	if ( (current_frame < 0) && taseditorConfig->findnoteSearchUp)
	{
		current_frame = movie_size;
	}
	while (true)
	{
		// move forward
		if (taseditorConfig->findnoteSearchUp)
		{
			current_frame--;
			if (current_frame < 0)
			{
				QMessageBox::information( this, tr("Find Note"), tr("Nothing was found!") );
				printf("Nothing was found\n");
				//MessageBox(taseditorWindow.hwndFindNote, "Nothing was found.", "Find Note", MB_OK);
				break;
			}
		}
		else
		{
			current_frame++;
			if (current_frame >= movie_size)
			{
				QMessageBox::information( this, tr("Find Note"), tr("Nothing was found!") );
				printf("Nothing was found\n");
				//MessageBox(taseditorWindow.hwndFindNote, "Nothing was found!", "Find Note", MB_OK);
				break;
			}
		}
		// scan marked frames
		cur_marker = markersManager->getMarkerAtFrame(current_frame);
		if (cur_marker)
		{
			QString haystack, needle;

			needle   = QString(markersManager->findNoteString);
			haystack = QString::fromStdString(markersManager->getNoteCopy(cur_marker));

			if (taseditorConfig->findnoteMatchCase)
			{
				result = haystack.indexOf( needle, 0, Qt::CaseSensitive ) >= 0;
				//result = (strstr(markersManager->getNoteCopy(cur_marker).c_str(), markersManager->findNoteString) != 0);
			}
			else
			{
				result = haystack.indexOf( needle, 0, Qt::CaseInsensitive ) >= 0;
//#ifdef WIN32
//				result = (StrStrI(markersManager->getNoteCopy(cur_marker).c_str(), markersManager->findNoteString) != 0);
//#else
//				result = (strcasestr(markersManager->getNoteCopy(cur_marker).c_str(), markersManager->findNoteString) != 0);
//#endif
			}
			if (result)
			{
				// found note containing searched string - jump there
				selection->jumpToFrame(current_frame);
				break;
			}
		}
	}
}
//----------------------------------------------------------------------------
//---- TAS Recent Project Menu Action
//----------------------------------------------------------------------------
TasRecentProjectAction::TasRecentProjectAction(QString desc, QWidget *parent)
	: QAction( desc, parent )
{
	QString txt;
	QFileInfo fi(desc);

	path = desc.toStdString();

	txt  = fi.fileName();
	txt += QString("\t");
	txt += desc;

	setText( txt );
}
//----------------------------------------------------------------------------
TasRecentProjectAction::~TasRecentProjectAction(void)
{
	//printf("Recent TAS Project Menu Action Deleted\n");
}
//----------------------------------------------------------------------------
void TasRecentProjectAction::activateCB(void)
{
	//printf("Activate Recent TAS Project: %s \n", path.c_str() );

	if ( tasWin )
	{
		tasWin->loadProject( path.c_str() );
	}
}
