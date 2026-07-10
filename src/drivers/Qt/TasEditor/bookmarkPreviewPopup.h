// bookmarkPreviewPopup.h
//

#pragma once

#include <memory>
#include <QDialog>
#include <QLabel>
#include <QTimer>
#include <QEvent>

class bookmarkPreviewPopup : public QDialog
{
   Q_OBJECT
	public:
	   bookmarkPreviewPopup( int index, QWidget *parent = nullptr );
	   ~bookmarkPreviewPopup(void);

	   int reloadImage(int index);

	   static int currentIndex(void);

	   static bookmarkPreviewPopup *currentInstance(void);

	protected:
		void changeEvent(QEvent *event) override;

	private:
		int loadImage(int index);

		void retranslateUi(void);

		int alpha;
		int imageIndex;
		bool actv;
		std::unique_ptr<unsigned char[]> screenShotRaster;
		QTimer *timer;
		QLabel *imgLbl, *descLbl;

		static bookmarkPreviewPopup *instance;

	public slots:
		void periodicUpdate(void);
		void imageIndexChanged(int);
};
