// AviOptionsDialog.cpp
//

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
//
// AviRecord.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <stdint.h>
#include <stdarg.h>
#include <string.h>
#include <string>

#ifdef WIN32
#include <windows.h>
#include <vfw.h>
#endif

#include <QFile>
#include <QDate>
#include <QLocale>
#include <QSysInfo>
#include <QObject>
#include <QHeaderView>
#include <QMessageBox>
#include <QInputDialog>

#include "fceu.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "version.h"
#include "common/os_utils.h"

#ifdef _USE_X264
#include "x264.h"
#endif
#ifdef _USE_X265
#include "x265.h"
#endif
#ifdef _USE_LIBAV
#ifdef __cplusplus
extern "C"
{
#include "libavutil/opt.h"
#include "libavutil/pixdesc.h"
#include "libavformat/avformat.h"
#include "libavcodec/avcodec.h"
#include "libswscale/swscale.h"
#include "libswresample/swresample.h"
}
#endif
#endif

#include "Qt/AviRecord.h"
#include "Qt/AviVideoCodec.h"
#include "Qt/avi/gwavi.h"
#include "Qt/nes_shm.h"
#include "Qt/throttle.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/fceuWrapper.h"

#include "Qt/AviOptionsDialog.h"
extern bool recordAudio;
extern int  videoFormat;
extern int  audioSampleRate;
extern int  aviDriver;

#ifdef _USE_LIBAV
LibavOptionsPage::LibavOptionsPage(QWidget *parent)
	: QWidget(parent)
{
	QLabel *lbl;
	QVBoxLayout *vbox, *vbox1;
	//QHBoxLayout *hbox;
	QGridLayout *grid;
	QPushButton *videoConfBtn, *audioConfBtn;

	g_config->getOption("SDL.AviRecordAudio", &recordAudio);

	LIBAV::setCodecFromConfig();

	vbox1 = new QVBoxLayout();

	videoGbox = new QGroupBox( tr("Video:") );
	audioGbox = new QGroupBox( tr("Audio:") );

	audioGbox->setCheckable(false);
	//audioGbox->setChecked( aviGetAudioEnable() );

	videoEncSel     = new QComboBox();
	audioEncSel     = new QComboBox();
	videoPixfmt     = new QComboBox();
	audioSamplefmt  = new QComboBox();
	audioSampleRate = new QComboBox();
	audioChanLayout = new QComboBox();

	vbox1->addWidget( videoGbox );
	vbox1->addWidget( audioGbox );

	vbox = new QVBoxLayout();
	videoGbox->setLayout(vbox);

	grid = new QGridLayout();
	vbox->addLayout(grid);
	lbl  = new QLabel( tr("Encoder:") );
	grid->addWidget( lbl, 0, 0);
	grid->addWidget( videoEncSel, 0, 1);
	lbl  = new QLabel( tr("Pixel Format:") );
	grid->addWidget( lbl, 1, 0);
	grid->addWidget( videoPixfmt, 1, 1);
	videoConfBtn = new QPushButton( tr("Options...") );
	grid->addWidget( videoConfBtn, 2, 1);

	vbox = new QVBoxLayout();
	audioGbox->setLayout(vbox);

	grid = new QGridLayout();
	vbox->addLayout(grid);
	lbl  = new QLabel( tr("Encoder:") );
	grid->addWidget( lbl, 0, 0);
	grid->addWidget( audioEncSel, 0, 1 );
	lbl  = new QLabel( tr("Sample Format:") );
	grid->addWidget( lbl, 1, 0);
	grid->addWidget( audioSamplefmt, 1, 1);
	lbl  = new QLabel( tr("Sample Rate:") );
	grid->addWidget( lbl, 2, 0);
	grid->addWidget( audioSampleRate, 2, 1);
	lbl  = new QLabel( tr("Channel Layout:") );
	grid->addWidget( lbl, 3, 0);
	grid->addWidget( audioChanLayout, 3, 1);
	audioConfBtn = new QPushButton( tr("Options...") );
	grid->addWidget( audioConfBtn, 4, 1);

	initCodecLists();

	setLayout(vbox1);

	connect(videoEncSel, SIGNAL(currentIndexChanged(int)), this, SLOT(videoCodecChanged(int)));
	connect(audioEncSel, SIGNAL(currentIndexChanged(int)), this, SLOT(audioCodecChanged(int)));

	connect(videoPixfmt    , SIGNAL(currentIndexChanged(int)), this, SLOT(videoPixelFormatChanged(int)));
	connect(audioSamplefmt , SIGNAL(currentIndexChanged(int)), this, SLOT(audioSampleFormatChanged(int)));
	connect(audioSampleRate, SIGNAL(currentIndexChanged(int)), this, SLOT(audioSampleRateChanged(int)));
	connect(audioChanLayout, SIGNAL(currentIndexChanged(int)), this, SLOT(audioChannelLayoutChanged(int)));

	connect(videoConfBtn, SIGNAL(clicked(void)), this, SLOT(openVideoCodecOptions(void)));
	connect(audioConfBtn, SIGNAL(clicked(void)), this, SLOT(openAudioCodecOptions(void)));

	//connect(audioGbox, SIGNAL(clicked(bool)), this, SLOT(includeAudioChanged(bool)));
	
	updateTimer = new QTimer(this);

	connect( updateTimer, &QTimer::timeout, this, &LibavOptionsPage::periodicUpdate );

	updateTimer->start(200);
}
//-----------------------------------------------------
LibavOptionsPage::~LibavOptionsPage(void)
{
	updateTimer->stop();
}
//----------------------------------------------------------------------------
void LibavOptionsPage::retranslateUi(void)
{
	if (videoGbox) videoGbox->setTitle(tr("Video"));
	if (audioGbox) audioGbox->setTitle(tr("Audio"));
}
//----------------------------------------------------------------------------
void LibavOptionsPage::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QWidget::changeEvent(event);
}
//-----------------------------------------------------
void LibavOptionsPage::periodicUpdate(void)
{
	audioGbox->setEnabled( recordAudio );
}
//-----------------------------------------------------
void LibavOptionsPage::initPixelFormatSelect(const char *codec_name)
{
	const AVCodec *c;
	const AVPixFmtDescriptor *desc;
	bool formatOk = false;

	c = avcodec_find_encoder_by_name( codec_name );

	videoPixfmt->clear();
	videoPixfmt->addItem( tr("Auto"), -1);

	if ( c == NULL )
	{
		return;
	}
	if ( c->pix_fmts )
	{
		int i=0; //, formatOk=0;
		while (c->pix_fmts[i] != -1)
		{
			desc = av_pix_fmt_desc_get( c->pix_fmts[i] );

			if ( desc )
			{
				//printf("Codec PIX_FMT: %i: %s 0x%04X\t-  %s\n", c->pix_fmts[i],
				//		desc->name, av_get_pix_fmt_loss(c->pix_fmts[i], AV_PIX_FMT_BGRA, 0), desc->alias);

				videoPixfmt->addItem( tr(desc->name), c->pix_fmts[i]);

				if ( LIBAV::video_st.pixelFormat == c->pix_fmts[i] )
				{
					videoPixfmt->setCurrentIndex( videoPixfmt->count() - 1 );
					formatOk = true;
				}
			}
			i++;
		}
		//if ( !formatOk )
		//{
		//	printf("CODEC Does Not Support PIX_FMT:%i  Changing to:%i\n", c->pix_fmt, codec->pix_fmts[0] );
		//	c->pix_fmt = codec->pix_fmts[0];
		//}
	}
	else
	{
		// List More Common Raw Video Formats Only
		desc = av_pix_fmt_desc_next( NULL );

		while ( desc != NULL )
		{
			AVPixelFormat pf;

			pf = av_pix_fmt_desc_get_id(desc);

			//printf("Codec PIX_FMT: %i: %s  0x%04X\t-  %s\n", 
			//		pf, desc->name, av_get_pix_fmt_loss(pf, AV_PIX_FMT_BGRA, 0), desc->alias);

			switch ( pf )
			{
				default:

				break;
				case  AV_PIX_FMT_YUV420P:   ///< planar YUV 4:2:0, 12bpp, (1 Cr & Cb sample per 2x2 Y samples)
				case  AV_PIX_FMT_YUYV422:   ///< packed YUV 4:2:2, 16bpp, Y0 Cb Y1 Cr
				case  AV_PIX_FMT_RGB24:     ///< packed RGB 8:8:8, 24bpp, RGBRGB...
				case  AV_PIX_FMT_BGR24:     ///< packed RGB 8:8:8, 24bpp, BGRBGR...
				case  AV_PIX_FMT_YUV422P:   ///< planar YUV 4:2:2, 16bpp, (1 Cr & Cb sample per 2x1 Y samples)
				case  AV_PIX_FMT_YUV444P:   ///< planar YUV 4:4:4, 24bpp, (1 Cr & Cb sample per 1x1 Y samples)
				case  AV_PIX_FMT_YUV410P:   ///< planar YUV 4:1:0,  9bpp, (1 Cr & Cb sample per 4x4 Y samples)
				case  AV_PIX_FMT_YUV411P:   ///< planar YUV 4:1:1, 12bpp, (1 Cr & Cb sample per 4x1 Y samples)
				case  AV_PIX_FMT_PAL8:      ///< 8 bits with AV_PIX_FMT_RGB32 palette
				case  AV_PIX_FMT_YUVJ420P:  ///< planar YUV 4:2:0, 12bpp, full scale (JPEG), deprecated in favor of AV_PIX_FMT_YUV420P and setting color_range
				case  AV_PIX_FMT_YUVJ422P:  ///< planar YUV 4:2:2, 16bpp, full scale (JPEG), deprecated in favor of AV_PIX_FMT_YUV422P and setting color_range
				case  AV_PIX_FMT_YUVJ444P:  ///< planar YUV 4:4:4, 24bpp, full scale (JPEG), deprecated in favor of AV_PIX_FMT_YUV444P and setting color_range
				case  AV_PIX_FMT_UYVY422:   ///< packed YUV 4:2:2, 16bpp, Cb Y0 Cr Y1
				case  AV_PIX_FMT_UYYVYY411: ///< packed YUV 4:1:1, 12bpp, Cb Y0 Y1 Cr Y2 Y3
				case  AV_PIX_FMT_NV12:      ///< planar YUV 4:2:0, 12bpp, 1 plane for Y and 1 plane for the UV components, which are interleaved (first byte U and the following byte V)
				case  AV_PIX_FMT_NV21:      ///< as above, but U and V bytes are swapped
				case  AV_PIX_FMT_ARGB:      ///< packed ARGB 8:8:8:8, 32bpp, ARGBARGB...
				case  AV_PIX_FMT_RGBA:      ///< packed RGBA 8:8:8:8, 32bpp, RGBARGBA...
				case  AV_PIX_FMT_ABGR:      ///< packed ABGR 8:8:8:8, 32bpp, ABGRABGR...
				case  AV_PIX_FMT_BGRA:      ///< packed BGRA 8:8:8:8, 32bpp, BGRABGRA...
					videoPixfmt->addItem( tr(desc->name), pf);

					if ( LIBAV::video_st.pixelFormat == pf )
					{
						videoPixfmt->setCurrentIndex( videoPixfmt->count() - 1 );
						formatOk = true;
					}
				break;
			}

			desc = av_pix_fmt_desc_next( desc );
		}
	}

	if ( !formatOk )
	{
		LIBAV::video_st.pixelFormat = -1;
	}

}
//-----------------------------------------------------
void LibavOptionsPage::initSampleFormatSelect( const char *codec_name )
{

	const AVCodec *c;
	bool formatOk = false;

	c = avcodec_find_encoder_by_name( codec_name );

	audioSamplefmt->clear();
	audioSamplefmt->addItem( tr("Auto"), -1);

	if ( c == NULL )
	{
		return;
	}
	if ( c->sample_fmts )
	{
		int i=0;
		const char *fmtName;

		while ( c->sample_fmts[i] != -1 )
		{
			fmtName = av_get_sample_fmt_name( c->sample_fmts[i] );

			if ( fmtName )
			{
				audioSamplefmt->addItem( tr(fmtName), c->sample_fmts[i] );

				if ( LIBAV::audio_st.sampleFormat == c->sample_fmts[i] )
				{
					audioSamplefmt->setCurrentIndex( audioSamplefmt->count() - 1 );
					formatOk = true;
				}
			}
			i++;
		}
	}
	if ( !formatOk )
	{
		LIBAV::audio_st.sampleFormat = -1;
	}
}
//-----------------------------------------------------
void LibavOptionsPage::initSampleRateSelect( const char *codec_name )
{

	const AVCodec *c;
	bool formatOk = false;

	c = avcodec_find_encoder_by_name( codec_name );

	audioSampleRate->clear();
	audioSampleRate->addItem( tr("Auto"), -1);

	if ( c == NULL )
	{
		return;
	}
	if ( c->supported_samplerates )
	{
		int i=0;
		char rateName[64];

		while ( c->supported_samplerates[i] != 0 )
		{
			snprintf( rateName, sizeof(rateName), "%i", c->supported_samplerates[i] );

			audioSampleRate->addItem( tr(rateName), c->supported_samplerates[i] );

			if ( LIBAV::audio_st.sampleRate == c->supported_samplerates[i] )
			{
				audioSampleRate->setCurrentIndex( audioSampleRate->count() - 1 );
				formatOk = true;
			}
			i++;
		}
	}
	if ( !formatOk )
	{
		LIBAV::audio_st.sampleRate = -1;
	}
}
//-----------------------------------------------------
void LibavOptionsPage::initChannelLayoutSelect( const char *codec_name )
{

	const AVCodec *c;
	bool formatOk = false;

	c = avcodec_find_encoder_by_name( codec_name );

	audioChanLayout->clear();
	audioChanLayout->addItem( tr("Auto"), -1);

	if ( c == NULL )
	{
		return;
	}
	#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
	if ( c->channel_layouts )
	{
		int i=0;
		char layoutDesc[256];

		while ( c->channel_layouts[i] != 0 )
		{
			av_get_channel_layout_string( layoutDesc, sizeof(layoutDesc), -1, c->channel_layouts[i] );

			audioChanLayout->addItem( tr(layoutDesc), (unsigned long long)c->channel_layouts[i] );

			if ( static_cast<uint64_t>(LIBAV::audio_st.chanLayout) == c->channel_layouts[i] )
			{
				audioChanLayout->setCurrentIndex( audioChanLayout->count() - 1 );
				formatOk = true;
			}
			i++;
		}

	}
	#else
	const AVChannelLayout *p = c->ch_layouts;

	while (p && p->nb_channels)
	{
		char layoutDesc[256];

		av_channel_layout_describe(p, layoutDesc, sizeof(layoutDesc));

		audioChanLayout->addItem( tr(layoutDesc), (unsigned long long)p->u.mask );

		if ( LIBAV::audio_st.chanLayout == p->u.mask )
		{
			audioChanLayout->setCurrentIndex( audioChanLayout->count() - 1 );
			formatOk = true;
		}
		p++;
	}
	#endif
	if ( !formatOk )
	{
		LIBAV::audio_st.chanLayout = -1;
	}
}
//-----------------------------------------------------
void LibavOptionsPage::initCodecLists(void)
{
	void *it = NULL;
	const AVCodec *c;
	const AVOutputFormat *ofmt;
	int compatible;

	ofmt = av_guess_format("avi", NULL, NULL);

	c = av_codec_iterate( &it );

	while ( c != NULL )
	{
		if ( av_codec_is_encoder(c) )
		{
			if ( c->type == AVMEDIA_TYPE_VIDEO )
			{
				compatible = avformat_query_codec( ofmt, c->id, FF_COMPLIANCE_NORMAL );
				//printf("Video Encoder: %i  %s   %s\t:%i\n", c->id, c->name, c->long_name, compatible);
				if ( compatible )
				{
					videoEncSel->addItem( tr(c->name), c->id );

					if ( strcmp( LIBAV::video_st.selEnc.c_str(), c->name ) == 0 )
					{
						videoEncSel->setCurrentIndex( videoEncSel->count() - 1 );
					}
				}
			}
			else if ( c->type == AVMEDIA_TYPE_AUDIO )
			{
				compatible = avformat_query_codec( ofmt, c->id, FF_COMPLIANCE_NORMAL );
				//printf("Audio Encoder: %i  %s   %s\t:%i\n", c->id, c->name, c->long_name, compatible);
				if ( compatible )
				{
					audioEncSel->addItem( tr(c->name), c->id );

					if ( strcmp( LIBAV::audio_st.selEnc.c_str(), c->name ) == 0 )
					{
						audioEncSel->setCurrentIndex( audioEncSel->count() - 1 );
					}
				}
			}
		}

		c = av_codec_iterate( &it );
	}

	initPixelFormatSelect( videoEncSel->currentText().toStdString().c_str() );
	initSampleFormatSelect( audioEncSel->currentText().toStdString().c_str() );
	initSampleRateSelect( audioEncSel->currentText().toStdString().c_str() );
	initChannelLayoutSelect( audioEncSel->currentText().toStdString().c_str() );

	videoEncSel->model()->sort(0, Qt::AscendingOrder);
	audioEncSel->model()->sort(0, Qt::AscendingOrder);
}
//-----------------------------------------------------
void LibavOptionsPage::includeAudioChanged(bool checked)
{
	aviSetAudioEnable( checked );
}
//-----------------------------------------------------
void LibavOptionsPage::videoCodecChanged(int idx)
{
	const AVCodec *c;

	LIBAV::video_st.selEnc = videoEncSel->currentText().toStdString().c_str();

	c = avcodec_find_encoder_by_name( LIBAV::video_st.selEnc.c_str() );

	if ( c )
	{
		g_config->setOption("SDL.AviFFmpegVideoCodec", c->name);	
		initPixelFormatSelect( c->name );
	}
}
//-----------------------------------------------------
void LibavOptionsPage::audioCodecChanged(int idx)
{
	const AVCodec *c;

	LIBAV::audio_st.selEnc = audioEncSel->currentText().toStdString().c_str();

	c = avcodec_find_encoder_by_name( LIBAV::audio_st.selEnc.c_str() );

	if ( c )
	{
		g_config->setOption("SDL.AviFFmpegAudioCodec", c->name);	

		initSampleFormatSelect( c->name );
		initSampleRateSelect( c->name );
		initChannelLayoutSelect( c->name );
	}
}
//-----------------------------------------------------
void LibavOptionsPage::videoPixelFormatChanged(int idx)
{
	LIBAV::video_st.pixelFormat = videoPixfmt->itemData(idx).toInt();

	printf("Selected Pixel Format: %i\n", LIBAV::video_st.pixelFormat );
	
	g_config->setOption("SDL.AviFFmpegVideoPixFmt", LIBAV::video_st.pixelFormat);	
}
//-----------------------------------------------------
void LibavOptionsPage::audioSampleFormatChanged(int idx)
{
	LIBAV::audio_st.sampleFormat = audioSamplefmt->itemData(idx).toInt();

	printf("Selected Sample Format: %i\n", LIBAV::audio_st.sampleFormat );
	
	g_config->setOption("SDL.AviFFmpegAudioSmpFmt", LIBAV::audio_st.sampleFormat);	
}
//-----------------------------------------------------
void LibavOptionsPage::audioSampleRateChanged(int idx)
{
	LIBAV::audio_st.sampleRate = audioSampleRate->itemData(idx).toInt();

	printf("Selected Sample Rate: %i\n", LIBAV::audio_st.sampleRate );
	
	g_config->setOption("SDL.AviFFmpegAudioSmpRate", LIBAV::audio_st.sampleRate);	
}
//-----------------------------------------------------
void LibavOptionsPage::audioChannelLayoutChanged(int idx)
{
	LIBAV::audio_st.chanLayout = audioChanLayout->itemData(idx).toInt();

	printf("Selected Channel Layout: 0x%X\n", LIBAV::audio_st.chanLayout );
	
	g_config->setOption("SDL.AviFFmpegAudioChanLayout", LIBAV::audio_st.chanLayout);	
}
//----------------------------------------------------------------------------
void LibavOptionsPage::openVideoCodecOptions(void)
{
	LibavEncOptWin *win = new LibavEncOptWin(0,this);

	win->show();
}
//----------------------------------------------------------------------------
void LibavOptionsPage::openAudioCodecOptions(void)
{
	LibavEncOptWin *win = new LibavEncOptWin(1,this);

	win->show();
}
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
LibavEncOptItem::LibavEncOptItem(QTreeWidgetItem *parent)
	: QTreeWidgetItem(parent)
{
	opt = NULL;
}
//----------------------------------------------------------------------------
LibavEncOptItem::~LibavEncOptItem(void)
{

}
//----------------------------------------------------------------------------
void LibavEncOptItem::setValueText(void)
{
	char stmp[256];
	uint8_t *s;

	stmp[0] = 0;

	s = NULL;

	if ( av_opt_get( obj, opt->name, 0, &s ) >= 0 )
	{
		if ( s != NULL )
		{
			FCEU_strlcpy( stmp, sizeof(stmp), (char*)s );
		}
	}
	if ( s != NULL )
	{
		av_free(s); s = NULL;
	}

	if ( units.size() > 0 )
	{
		switch ( opt->type )
		{
			case AV_OPT_TYPE_FLAGS:
			{
				int64_t i,j;

				j=0;
				if ( av_opt_get_int( obj, opt->name, 0, &i ) >= 0 )
				{
					for (size_t x=0; x<units.size(); x++)
					{
						if ( units[x]->default_val.i64 & i )
						{
							char stmp2[128];
							snprintf( stmp2, sizeof(stmp2), "%s", units[x]->name );
							if (j>0)
							{
								safe_strcat( stmp, sizeof(stmp), ",");
							}
							else
							{
								safe_strcat( stmp, sizeof(stmp), " (");
							}
							safe_strcat( stmp, sizeof(stmp), stmp2 );
							j++;
						}
					}
					if ( j > 0 )
					{
						safe_strcat( stmp, sizeof(stmp), ")");
					}
				}
			}
			break;
			case AV_OPT_TYPE_INT:
			case AV_OPT_TYPE_INT64:
			#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(55, 58, 100)
			case AV_OPT_TYPE_UINT64:
			#endif
			{
				int64_t i;

				if ( av_opt_get_int( obj, opt->name, 0, &i ) >= 0 )
				{
					for (size_t x=0; x<units.size(); x++)
					{
						if ( units[x]->default_val.i64 == i )
						{
							char stmp2[128];
							snprintf( stmp2, sizeof(stmp2), " (%s)", units[x]->name );
							safe_strcat( stmp, sizeof(stmp), stmp2 );
							break;
						}
					}
				}
			}
			break;
			default:
				// Nothing to add
			break;
		}
	}
	setText(1, QString::fromStdString(stmp));
}
//----------------------------------------------------------------------------
LibavEncOptWin::LibavEncOptWin(int type, QWidget *parent)
	: QDialog(parent)
{
	QVBoxLayout *mainLayout;
	QHBoxLayout *hbox;
	QPushButton *closeButton, *resetDefaults;
	QTreeWidgetItem *itemHdr = NULL;
	QTreeWidgetItem *groupItem = NULL;
	LibavEncOptItem *item = NULL;
	const AVCodec *codec;
	const AVOption *opt;
	bool useOpt, newOpt;
	char title[128];
	void *obj = NULL, *ctx_child = NULL;

	this->type = type;

	if ( type )
	{
		codec_name = LIBAV::audio_st.selEnc.c_str();
		snprintf( title, sizeof(title), "%s Audio Encoder Configuration", codec_name );
	}
	else
	{
		codec_name = LIBAV::video_st.selEnc.c_str();
		snprintf( title, sizeof(title), "%s Video Encoder Configuration", codec_name );
	}
	setWindowTitle( title );
	resize(512, 512);

	/* find the video encoder */
	codec = avcodec_find_encoder_by_name(codec_name);

	ctx = NULL;
	if (codec != NULL)
	{
		//printf("CODEC: %s\n", codec->name );

		ctx = avcodec_alloc_context3(codec);

		LIBAV::loadCodecConfig( type, codec_name, ctx );

		//av_opt_show2( (void*)ctx, NULL, AV_OPT_FLAG_VIDEO_PARAM, 0 );

		//ctx_child = av_opt_child_next( ctx, ctx_child );

		//while ( ctx_child != NULL )
		//{
		//	av_opt_show2( ctx_child, NULL, AV_OPT_FLAG_VIDEO_PARAM, 0 );

		//	ctx_child = av_opt_child_next( ctx, ctx_child );
		//}
	}
	obj = ctx;

	mainLayout = new QVBoxLayout();

	tree = new QTreeWidget(this);

	tree->setColumnCount(3);
	tree->setSelectionMode( QAbstractItemView::SingleSelection );
	tree->setAlternatingRowColors(true);

	itemHdr = new QTreeWidgetItem();
	itemHdr->setText(0, QString::fromStdString("Option"));
	itemHdr->setText(1, QString::fromStdString("Value"));
	itemHdr->setText(2, QString::fromStdString("Desc"));
	itemHdr->setTextAlignment(0, Qt::AlignLeft);
	itemHdr->setTextAlignment(1, Qt::AlignLeft);
	itemHdr->setTextAlignment(2, Qt::AlignLeft);

	tree->setHeaderItem(itemHdr);

	tree->horizontalScrollBar()->setEnabled(true);
	tree->setHorizontalScrollBarPolicy(Qt::ScrollBarAsNeeded);

	tree->header()->setStretchLastSection(true);
	tree->header()->setSectionResizeMode(QHeaderView::ResizeToContents);

	//printf("CTX Class: %s\n", ctx->av_class->class_name );
	obj = ctx;
	ctx_child = NULL;

	while ( obj != NULL )
	{
		const char *groupName = (*static_cast<AVClass**>(obj))->class_name;

		//printf("OBJ Class: %s\n", groupName);

		groupItem = new QTreeWidgetItem();
		tree->addTopLevelItem(groupItem);

		groupItem->setText(0, QString::fromStdString(groupName));

		opt = av_opt_next( obj, NULL );

		while ( opt != NULL )
		{
			useOpt = (opt->name != NULL) &&
					(opt->type != AV_OPT_TYPE_BINARY) &&
					(opt->type != AV_OPT_TYPE_DICT);

			if ( type )
			{
				useOpt = useOpt &&
						(opt->flags & AV_OPT_FLAG_ENCODING_PARAM) &&
						(opt->flags & AV_OPT_FLAG_AUDIO_PARAM);
			}
			else
			{
				useOpt = useOpt &&
						(opt->flags & AV_OPT_FLAG_ENCODING_PARAM) &&
						(opt->flags & AV_OPT_FLAG_VIDEO_PARAM);
			}
			newOpt = true;

			if ( item )
			{
				if ( opt->unit && item->opt->unit )
				{
					if ( strcmp( opt->unit, item->opt->unit ) == 0 )
					{
						newOpt = false;
					}
				}
			}

			if ( useOpt )
			{
				if ( newOpt )
				{
					item = new LibavEncOptItem();

					item->obj = obj;
					item->opt = opt;
					item->setText(0, QString::fromStdString(opt->name));
					//item->setText(1, QString::fromStdString("Value"));
					if ( opt->help )
					{
						item->setText(2, QString::fromStdString(opt->help));
						item->setToolTip( 0, tr(opt->help) );
						item->setToolTip( 1, tr(opt->help) );
					}

					item->setValueText();
					item->setTextAlignment(0, Qt::AlignLeft);
					item->setTextAlignment(1, Qt::AlignLeft);
					item->setTextAlignment(2, Qt::AlignLeft);

					groupItem->addChild(item);
				}
				else
				{
					if ( item )
					{
						item->units.push_back(opt);
						item->setValueText();
					}
				}
				//printf("Option: %s - %i - %s - %s\n", opt->name, opt->type, opt->unit, opt->help);

				//if ( opt->type == AV_OPT_TYPE_FLAGS )
				//{
				//	printf("   Value: %llx \n", (unsigned long long)opt->default_val.i64 );
				//}
				//else if ( opt->type == AV_OPT_TYPE_CONST )
				//{
				//	printf("   Value: %llx \n", (unsigned long long)opt->default_val.i64 );
				//}
			}
			opt = av_opt_next( obj, opt );
		}
		obj = ctx_child = av_opt_child_next( ctx, ctx_child );
	}
	sortItems();

	tree->resizeColumnToContents(2);

	//connect( tree, SIGNAL(itemDoubleClicked(QTreeWidgetItem*,int)), this, SLOT(hotKeyDoubleClicked(QTreeWidgetItem*,int) ) );
	connect( tree, SIGNAL(itemActivated(QTreeWidgetItem*,int)), this, SLOT(itemChangeActivated(QTreeWidgetItem*,int) ) );

	mainLayout->addWidget(tree);

	resetDefaults = new QPushButton( tr("Restore Defaults") );
	resetDefaults->setIcon(style()->standardIcon(QStyle::SP_DialogResetButton));
	connect(resetDefaults, SIGNAL(clicked(void)), this, SLOT(resetDefaultsCB(void)));

	closeButton = new QPushButton( tr("Close") );
	closeButton->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
	connect(closeButton, SIGNAL(clicked(void)), this, SLOT(closeWindow(void)));

	hbox = new QHBoxLayout();
	hbox->addWidget( resetDefaults, 1 );
	hbox->addStretch(5);
	hbox->addWidget( closeButton, 1 );
	mainLayout->addLayout( hbox );

	setLayout(mainLayout);
}
//----------------------------------------------------------------------------
LibavEncOptWin::~LibavEncOptWin(void)
{
	//printf("Destroy Encoder Options Config Window\n");

	LIBAV::saveCodecConfig(type, codec_name, ctx);
}
//----------------------------------------------------------------------------
void LibavEncOptWin::closeEvent(QCloseEvent *event)
{
	//printf("Encoder Options Close Window Event\n");
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void LibavEncOptWin::retranslateUi(void)
{
	// Libav encoder option tree has dynamic keys; left as-is.
}
//----------------------------------------------------------------------------
void LibavEncOptWin::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
void LibavEncOptWin::closeWindow(void)
{
	//printf("Close Window\n");
	done(0);
	deleteLater();
}
//-----------------------------------------------------
void LibavEncOptWin::resetDefaultsCB(void)
{
	void *obj = NULL;

	av_opt_set_defaults(ctx);

	obj = av_opt_child_next( ctx, obj );

	while (obj != NULL)
	{
		av_opt_set_defaults(obj);

		obj = av_opt_child_next( ctx, obj );
	}
	updateItems();
}
//-----------------------------------------------------
void LibavEncOptWin::sortItems(void)
{
	QTreeWidgetItem *groupItem;

	for (int i=0; i<tree->topLevelItemCount(); i++)
	{
		groupItem = tree->topLevelItem(i);

		groupItem->sortChildren(0, Qt::AscendingOrder);
	}
	tree->viewport()->update();
}
//-----------------------------------------------------
void LibavEncOptWin::updateItems(void)
{
	QTreeWidgetItem *groupItem;
	LibavEncOptItem *item = NULL;

	for (int i=0; i<tree->topLevelItemCount(); i++)
	{
		groupItem = tree->topLevelItem(i);

		for (int j=0; j<groupItem->childCount(); j++)
		{
			item = static_cast<LibavEncOptItem*>(groupItem->child(j));
			item->setValueText();
		}
	}
	tree->viewport()->update();
}
//-----------------------------------------------------
void LibavEncOptWin::itemChangeActivated( QTreeWidgetItem *itemBase, int col)
{
	LibavEncOptItem *item = NULL;
	LibavEncOptInputWin *win;

	if ( itemBase->childCount() > 0 )
	{
		return;
	}

	item = static_cast<LibavEncOptItem*>(itemBase);

	if ( item->opt == NULL )
	{
		return;
	}
	win = new LibavEncOptInputWin( item, this );

	win->show();

	connect( win, SIGNAL(finished(int)), this, SLOT(editWindowFinished(int)) );
}
//-----------------------------------------------------
void LibavEncOptWin::editWindowFinished(int result)
{
	updateItems();
}
//-----------------------------------------------------
//-----------------------------------------------------
//-----------------------------------------------------
//----------------------------------------------------------------------------
LibavEncOptInputWin::LibavEncOptInputWin( LibavEncOptItem *itemIn, QWidget *parent )
	: QDialog(parent)
{
	const AVOption *opt;
	QVBoxLayout *mainLayout;
	QHBoxLayout *hbox;
	QGridLayout *grid;
	QLabel *lbl;
	char stmp[128];
	void *obj;

	item = itemIn;
	opt  = item->opt;
	obj  = item->obj;

	setWindowTitle("Set Value");

	mainLayout = new QVBoxLayout();
	grid       = new QGridLayout();

	setLayout( mainLayout );
	mainLayout->addLayout(grid);
	grid->addWidget( new QLabel( tr("Name:")   ), 0, 0 );
	grid->addWidget( new QLabel( tr(opt->name) ), 0, 1 );

	if ( opt->help )
	{
		lbl = new QLabel( tr(opt->help) );
		lbl->setWordWrap(true);

		grid->addWidget( new QLabel( tr("Desc:")   ), 1, 0 );
		grid->addWidget( lbl, 1, 1 );
	}
	combo = NULL;
	intEntry = NULL;
	floatEntry = NULL;
	numEntry = NULL;
	denEntry = NULL;
	strEntry = NULL;

	switch ( opt->type )
	{
		case AV_OPT_TYPE_INT:
		case AV_OPT_TYPE_INT64:
		#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(55, 58, 100)
		case AV_OPT_TYPE_UINT64:
		#endif
		{
			int64_t val;

			av_opt_get_int( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Range:")   ), 2, 0 );

			snprintf( stmp, sizeof(stmp), "[ %.0f, %.0f ]", opt->min, opt->max );

			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			grid->addWidget( new QLabel( tr("Default:")   ), 3, 0 );

			snprintf( stmp, sizeof(stmp), "%lli", (long long)opt->default_val.i64 );

			grid->addWidget( new QLabel( tr(stmp)   ), 3, 1 );

			grid->addWidget( new QLabel( tr("Value:")   ), 4, 0 );

			if ( item->units.size() > 0 )
			{
				combo = new QComboBox();

				grid->addWidget( combo, 4, 1 );

				for (size_t i=0; i<item->units.size(); i++)
				{
					if ( item->units[i]->help )
					{
						snprintf( stmp, sizeof(stmp), "%3lli:  %s  -  %s", (long long)item->units[i]->default_val.i64,
							item->units[i]->name, item->units[i]->help );
					}
					else
					{
						snprintf( stmp, sizeof(stmp), "%3lli:  %s", (long long)item->units[i]->default_val.i64,
							item->units[i]->name );
					}

					combo->addItem( tr(stmp), (const long long)item->units[i]->default_val.i64 );

					if ( val == item->units[i]->default_val.i64 )
					{
						combo->setCurrentIndex( combo->count() - 1 );
					}
				}
			}
			else
			{
				intEntry = new QSpinBox();
				intEntry->setValue( val );
				intEntry->setRange( (int)(opt->min), (int)(opt->max) );

				grid->addWidget( intEntry, 4, 1 );
			}
		}
		break;
		case AV_OPT_TYPE_FLOAT:
		case AV_OPT_TYPE_DOUBLE:
		{
			double val;

			av_opt_get_double( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Range:")   ), 2, 0 );

			snprintf( stmp, sizeof(stmp), "[ %e, %e ]", opt->min, opt->max );

			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			grid->addWidget( new QLabel( tr("Default:")   ), 3, 0 );

			snprintf( stmp, sizeof(stmp), "%f", opt->default_val.dbl );

			grid->addWidget( new QLabel( tr(stmp)   ), 3, 1 );

			grid->addWidget( new QLabel( tr("Value:")   ), 4, 0 );

			floatEntry = new QDoubleSpinBox();
			floatEntry->setValue( val );
			floatEntry->setRange( opt->min, opt->max );

			grid->addWidget( floatEntry, 4, 1 );
		}
		break;
		case AV_OPT_TYPE_STRING:
		{
			uint8_t *val = NULL;

			av_opt_get( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Default:")   ), 2, 0 );

			stmp[0] = 0;

			if ( opt->default_val.str )
			{
				snprintf( stmp, sizeof(stmp), "%s", opt->default_val.str );
			}
			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			grid->addWidget( new QLabel( tr("Value:")   ), 3, 0 );

			strEntry = new QLineEdit();

			if ( val )
			{
				strEntry->setText( tr( (const char*)val ) );
				av_free(val); val = NULL;
			}
			grid->addWidget( strEntry, 3, 1 );
		}
		break;
		case AV_OPT_TYPE_RATIONAL:
		{
			AVRational val;

			av_opt_get_q( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Default:")   ), 2, 0 );

			snprintf( stmp, sizeof(stmp), "%i/%i", opt->default_val.q.num, opt->default_val.q.den );

			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			grid->addWidget( new QLabel( tr("Numerator:")   ), 3, 0 );

			numEntry = new QSpinBox();
			numEntry->setValue( val.num );
			numEntry->setRange( 0, 0x7FFFFFFF );

			grid->addWidget( numEntry, 3, 1 );

			grid->addWidget( new QLabel( tr("Denominator:")   ), 4, 0 );

			denEntry = new QSpinBox();
			denEntry->setValue( val.den );
			denEntry->setRange( 1, 0x7FFFFFFF );

			grid->addWidget( denEntry, 4, 1 );
		}
		break;
		case AV_OPT_TYPE_BOOL:
		{
			int64_t val;
			QCheckBox *c;

			av_opt_get_int( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Default:")   ), 2, 0 );

			snprintf( stmp, sizeof(stmp), "%s", opt->default_val.i64 ? "true" : "false" );

			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			grid->addWidget( new QLabel( tr("Value:")   ), 3, 0 );

			c = new QCheckBox( tr("Checked=true") );

			c->setChecked( val != 0 );

			chkBox.push_back(c);

			grid->addWidget( c, 3, 1 );
		}
		break;
		case AV_OPT_TYPE_FLAGS:
		{
			int64_t val;
			QCheckBox *c;

			av_opt_get_int( obj, opt->name, 0, &val );

			grid->addWidget( new QLabel( tr("Default:")   ), 2, 0 );

			snprintf( stmp, sizeof(stmp), "0x%08llX", (unsigned long long)opt->default_val.i64 );

			grid->addWidget( new QLabel( tr(stmp)   ), 2, 1 );

			if ( item->units.size() > 0 )
			{
				int row, col;

				for (size_t i=0; i<item->units.size(); i++)
				{
					snprintf( stmp, sizeof(stmp), "%s", item->units[i]->name );

					c = new QCheckBox( tr(stmp) );

					if ( item->units[i]->help )
					{
						c->setToolTip( tr(item->units[i]->help) );
					}

					c->setChecked( (val & item->units[i]->default_val.i64) ? true : false );

					chkBox.push_back(c);
				}

				row = col = 0;
				for (size_t i=0; i<chkBox.size(); i++)
				{
					grid->addWidget( chkBox[i], row+3, col );
					
					row++;
					if ( row >= 8 )
					{
						row = 0;
						col++;
					}
				}
			}
		}
		break;
		default:

		break;
	}

	    okButton  = new QPushButton( tr("Apply") );
	cancelButton  = new QPushButton( tr("Cancel") );
	resetDefaults = new QPushButton( tr("Reset") );

	     okButton->setIcon(style()->standardIcon(QStyle::SP_DialogApplyButton));
	 cancelButton->setIcon(style()->standardIcon(QStyle::SP_DialogCancelButton));
	resetDefaults->setIcon(style()->standardIcon(QStyle::SP_DialogResetButton));

	hbox = new QHBoxLayout();
	hbox->addWidget( resetDefaults, 1 );
	hbox->addWidget(  cancelButton, 1 );
	hbox->addStretch(5);
	hbox->addWidget(      okButton, 1 );
	mainLayout->addLayout(hbox);

	connect(      okButton, SIGNAL(clicked(void)), this, SLOT(applyChanges(void))    );
	connect(  cancelButton, SIGNAL(clicked(void)), this, SLOT(closeWindow(void))     );
	connect( resetDefaults, SIGNAL(clicked(void)), this, SLOT(resetDefaultsCB(void)) );
}
//-----------------------------------------------------
LibavEncOptInputWin::~LibavEncOptInputWin(void)
{

}
//----------------------------------------------------------------------------
void LibavEncOptInputWin::closeEvent(QCloseEvent *event)
{
	//printf("Encoder Options Close Window Event\n");
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void LibavEncOptInputWin::retranslateUi(void)
{
	if (okButton) okButton->setText(tr("OK"));
	if (cancelButton) cancelButton->setText(tr("Cancel"));
	if (resetDefaults) resetDefaults->setText(tr("Reset Defaults"));
}
//----------------------------------------------------------------------------
void LibavEncOptInputWin::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
void LibavEncOptInputWin::closeWindow(void)
{
	//printf("Close Window\n");
	done(0);
	deleteLater();
}
//-----------------------------------------------------
void LibavEncOptInputWin::applyChanges(void)
{
	switch ( item->opt->type )
	{
		case AV_OPT_TYPE_INT:
		case AV_OPT_TYPE_INT64:
		#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(55, 58, 100)
		case AV_OPT_TYPE_UINT64:
		#endif
		{
			if ( intEntry )
			{
				av_opt_set_int( item->obj, item->opt->name, intEntry->value(), 0 );
			}

			if ( combo )
			{
				av_opt_set_int( item->obj, item->opt->name, 
						combo->currentData().toInt(), 0 );
			}
		}
		break;
		case AV_OPT_TYPE_FLOAT:
		case AV_OPT_TYPE_DOUBLE:
		{
			if ( floatEntry )
			{
				av_opt_set_double( item->obj, item->opt->name, floatEntry->value(), 0 );
			}
		}
		break;
		case AV_OPT_TYPE_STRING:
		{
			if ( strEntry )
			{
				av_opt_set( item->obj, item->opt->name, strEntry->text().toStdString().c_str(), 0 );
			}
		}
		break;
		case AV_OPT_TYPE_RATIONAL:
		{
			AVRational q;

			q.num = 0;
			q.den = 1;

			if ( numEntry )
			{
				q.num = numEntry->value();
			}

			if ( denEntry )
			{
				q.den = denEntry->value();
			}

			av_opt_set_q( item->obj, item->opt->name, q, 0 );
		}
		break;
		case AV_OPT_TYPE_BOOL:
		{
			if ( chkBox.size() > 0 )
			{
				av_opt_set_int( item->obj, item->opt->name, chkBox[0]->isChecked(), 0 );
			}
		}
		break;
		case AV_OPT_TYPE_FLAGS:
		{
			if ( chkBox.size() > 0 )
			{
				int64_t i64=0;

				for (size_t i=0; i<chkBox.size(); i++)
				{
					if ( chkBox[i]->isChecked() )
					{
						i64 |= item->units[i]->default_val.i64;
					}
				}
				av_opt_set_int( item->obj, item->opt->name, i64, 0 );
			}
		}
		break;
		default:
		break;
	}

	done(0);
	deleteLater();
}
//-----------------------------------------------------
void LibavEncOptInputWin::resetDefaultsCB(void)
{
	switch ( item->opt->type )
	{
		case AV_OPT_TYPE_INT:
		case AV_OPT_TYPE_INT64:
		#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(55, 58, 100)
		case AV_OPT_TYPE_UINT64:
		#endif
		{
			if ( intEntry )
			{
				intEntry->setValue( item->opt->default_val.i64 );
			}

			if ( combo )
			{
				for (int i=0; i<combo->count(); i++)
				{
					if ( combo->itemData(i).toInt() == item->opt->default_val.i64 )
					{
						combo->setCurrentIndex(i);
					}
				}
			}
		}
		break;
		case AV_OPT_TYPE_FLOAT:
		case AV_OPT_TYPE_DOUBLE:
		{
			if ( floatEntry )
			{
				floatEntry->setValue( item->opt->default_val.dbl );
			}
		}
		break;
		case AV_OPT_TYPE_STRING:
		{
			if ( strEntry )
			{
				if ( item->opt->default_val.str )
				{
					strEntry->setText( tr(item->opt->default_val.str) );
				}
				else
				{
					strEntry->clear();
				}
			}
		}
		break;
		case AV_OPT_TYPE_RATIONAL:
		{
			if ( numEntry )
			{
				numEntry->setValue( item->opt->default_val.q.num );
			}

			if ( denEntry )
			{
				denEntry->setValue( item->opt->default_val.q.den );
			}
		}
		break;
		case AV_OPT_TYPE_BOOL:
		{
			if ( chkBox.size() > 0 )
			{
				chkBox[0]->setChecked( item->opt->default_val.i64 != 0 );
			}
		}
		break;
		case AV_OPT_TYPE_FLAGS:
		{
			if ( chkBox.size() > 0 )
			{
				for (size_t i=0; i<chkBox.size(); i++)
				{
					chkBox[i]->setChecked( (item->opt->default_val.i64 & item->units[i]->default_val.i64) ? true : false );
				}
			}
		}
		break;
		default:

		break;
	}
}
//-----------------------------------------------------
#endif
//**************************************************************************************
LibgwaviOptionsPage::LibgwaviOptionsPage(QWidget *parent)
	: QWidget(parent)
{
	QLabel *lbl;
	QVBoxLayout *vbox, *vbox1;
	//QHBoxLayout *hbox;
	QGridLayout *grid;
	QPushButton *videoConfBtn, *audioConfBtn;

	g_config->getOption("SDL.AviRecordAudio", &recordAudio);

	vbox1 = new QVBoxLayout();

	videoGbox = new QGroupBox( tr("Video:") );
	audioGbox = new QGroupBox( tr("Audio:") );

	audioGbox->setCheckable(false);
	//audioGbox->setChecked( aviGetAudioEnable() );

	videoEncSel     = new QComboBox();
	audioEncSel     = new QComboBox();
	videoPixfmt     = new QComboBox();
	audioSamplefmt  = new QComboBox();
	audioSampleRate = new QComboBox();
	audioChanLayout = new QComboBox();

	vbox1->addWidget( videoGbox );
	vbox1->addWidget( audioGbox );

	vbox = new QVBoxLayout();
	videoGbox->setLayout(vbox);

	grid = new QGridLayout();
	vbox->addLayout(grid);
	lbl  = new QLabel( tr("Encoder:") );
	grid->addWidget( lbl, 0, 0);
	grid->addWidget( videoEncSel, 0, 1);
	lbl  = new QLabel( tr("Pixel Format:") );
	grid->addWidget( lbl, 1, 0);
	grid->addWidget( videoPixfmt, 1, 1);
	videoConfBtn = new QPushButton( tr("Options...") );
	grid->addWidget( videoConfBtn, 2, 1);
	videoConfBtn->setEnabled(false);

	vbox = new QVBoxLayout();
	audioGbox->setLayout(vbox);

	grid = new QGridLayout();
	vbox->addLayout(grid);
	lbl  = new QLabel( tr("Encoder:") );
	grid->addWidget( lbl, 0, 0);
	grid->addWidget( audioEncSel, 0, 1 );
	lbl  = new QLabel( tr("Sample Format:") );
	grid->addWidget( lbl, 1, 0);
	grid->addWidget( audioSamplefmt, 1, 1);
	lbl  = new QLabel( tr("Sample Rate:") );
	grid->addWidget( lbl, 2, 0);
	grid->addWidget( audioSampleRate, 2, 1);
	lbl  = new QLabel( tr("Channel Layout:") );
	grid->addWidget( lbl, 3, 0);
	grid->addWidget( audioChanLayout, 3, 1);
	audioConfBtn = new QPushButton( tr("Options...") );
	grid->addWidget( audioConfBtn, 4, 1);
	audioConfBtn->setEnabled(false);

	initCodecLists();

	setLayout(vbox1);

	connect(videoEncSel, SIGNAL(currentIndexChanged(int)), this, SLOT(videoCodecChanged(int)));
	connect(audioEncSel, SIGNAL(currentIndexChanged(int)), this, SLOT(audioCodecChanged(int)));

	connect(videoPixfmt    , SIGNAL(currentIndexChanged(int)), this, SLOT(videoPixelFormatChanged(int)));
	connect(audioSamplefmt , SIGNAL(currentIndexChanged(int)), this, SLOT(audioSampleFormatChanged(int)));
	connect(audioSampleRate, SIGNAL(currentIndexChanged(int)), this, SLOT(audioSampleRateChanged(int)));
	connect(audioChanLayout, SIGNAL(currentIndexChanged(int)), this, SLOT(audioChannelLayoutChanged(int)));

	connect(videoConfBtn, SIGNAL(clicked(void)), this, SLOT(openVideoCodecOptions(void)));
	connect(audioConfBtn, SIGNAL(clicked(void)), this, SLOT(openAudioCodecOptions(void)));

	updateTimer = new QTimer(this);

	connect( updateTimer, &QTimer::timeout, this, &LibgwaviOptionsPage::periodicUpdate );

	updateTimer->start(200);
}
//-----------------------------------------------------
LibgwaviOptionsPage::~LibgwaviOptionsPage(void)
{
	updateTimer->stop();
}
//----------------------------------------------------------------------------
void LibgwaviOptionsPage::retranslateUi(void)
{
	if (videoGbox) videoGbox->setTitle(tr("Video"));
	if (audioGbox) audioGbox->setTitle(tr("Audio"));
}
//----------------------------------------------------------------------------
void LibgwaviOptionsPage::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QWidget::changeEvent(event);
}
//-----------------------------------------------------
void LibgwaviOptionsPage::periodicUpdate(void)
{
	audioGbox->setEnabled( recordAudio );
}
//-----------------------------------------------------
void LibgwaviOptionsPage::initCodecLists(void)
{
	int videoEncoder = aviGetSelVideoFormat();

	videoEncSel->addItem( tr("RGB24 (Uncompressed)"), AVI_RGB24 );
	videoEncSel->addItem( tr("I420  (YUV 4:2:0)")   , AVI_I420  );
	#ifdef _USE_X264
	videoEncSel->addItem( tr("X264  (H.264)")   , AVI_X264  );
	#endif
	#ifdef _USE_X265
	videoEncSel->addItem( tr("X265  (H.265)")   , AVI_X265  );
	#endif
	#ifdef WIN32
	videoEncSel->addItem( tr("VfW (Video for Windows)"), AVI_VFW);
	#endif

	for (int i=0; i<videoEncSel->count(); i++)
	{
		if ( videoEncoder == videoEncSel->itemData(i).toInt() )
		{
			videoEncSel->setCurrentIndex(i); break;
		}
	}
	audioEncSel->addItem( tr("Raw PCM (Uncompressed)"), 0 );

	int audioEncoder = audioEncSel->currentData().toInt();

	initPixelFormatSelect(videoFormat);
	initSampleFormatSelect(audioEncoder);
	initSampleRateSelect(audioEncoder);
	initChannelLayoutSelect(audioEncoder);
}
//-----------------------------------------------------
void LibgwaviOptionsPage::initPixelFormatSelect( int encoder )
{
	videoPixfmt->clear();
	videoPixfmt->addItem( tr("Auto"), -1 );

	switch ( encoder )
	{
		default:
		case AVI_I420:
			videoPixfmt->addItem( tr("YUV 420"), AVI_I420 );
		break;
#ifdef _USE_X264
		case AVI_X264:
			videoPixfmt->addItem( tr("YUV 420"), AVI_I420 );
		break;
#endif
#ifdef _USE_X265
		case AVI_X265:
			videoPixfmt->addItem( tr("YUV 420"), AVI_I420 );
		break;
#endif
#ifdef WIN32
		case AVI_VFW:
#endif
		case AVI_RGB24:
			videoPixfmt->addItem( tr("RGB24"), AVI_RGB24 );
		break;
	}
}
//-----------------------------------------------------
void LibgwaviOptionsPage::initSampleFormatSelect( int encoder )
{
	audioSamplefmt->clear();
	audioSamplefmt->addItem( tr("Auto"), -1 );
	audioSamplefmt->addItem( tr("S16 - Signed 16 Bit") ,  0 );
}
//-----------------------------------------------------
void LibgwaviOptionsPage::initSampleRateSelect( int encoder )
{
	audioSampleRate->clear();
	audioSampleRate->addItem( tr("Auto"), -1 );
}
//-----------------------------------------------------
void LibgwaviOptionsPage::initChannelLayoutSelect( int encoder )
{
	audioChanLayout->clear();
	audioChanLayout->addItem( tr("Auto"), -1 );
	audioChanLayout->addItem( tr("Mono"),  0 );
}
//-----------------------------------------------------
void LibgwaviOptionsPage::videoCodecChanged(int idx)
{
	aviSetSelVideoFormat( videoEncSel->currentData().toInt() );

	initPixelFormatSelect(videoFormat);
}
//-----------------------------------------------------
void LibgwaviOptionsPage::audioCodecChanged(int idx)
{
	int audioEncoder = audioEncSel->currentData().toInt();

	initSampleFormatSelect(audioEncoder);
	initSampleRateSelect(audioEncoder);
	initChannelLayoutSelect(audioEncoder);
}
//-----------------------------------------------------
void LibgwaviOptionsPage::openVideoCodecOptions(void)
{

}
//-----------------------------------------------------
void LibgwaviOptionsPage::openAudioCodecOptions(void)
{

}
//-----------------------------------------------------
void LibgwaviOptionsPage::videoPixelFormatChanged(int idx)
{

}
//-----------------------------------------------------
void LibgwaviOptionsPage::audioSampleFormatChanged(int idx)
{

}
//-----------------------------------------------------
void LibgwaviOptionsPage::audioSampleRateChanged(int idx)
{

}
//-----------------------------------------------------
void LibgwaviOptionsPage::audioChannelLayoutChanged(int idx)
{

}
//-----------------------------------------------------
//**************************************************************************************
