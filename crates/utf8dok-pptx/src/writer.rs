//! PPTX generation from slide data.
//!
//! This module generates PPTX files from slides and templates.

use crate::constants::*;
use crate::error::Result;
use crate::layout::LayoutMapping;
use crate::slide::{ListContent, ListItem, Slide, SlideContent, TextContent, TextRun};
use crate::slide_contract::SlideContract;
use crate::template::PotxTemplate;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// PPTX document writer
pub struct PptxWriter {
    /// Slide contract for layout mappings
    contract: SlideContract,

    /// Template (if any)
    template: Option<PotxTemplate>,

    /// Layout mapping
    layout_mapping: LayoutMapping,

    /// Slides to render
    slides: Vec<Slide>,

    /// Media files (path -> (rel_id, content_type))
    media: Vec<MediaItem>,

    /// Presentation title
    title: Option<String>,

    /// Presentation author
    author: Option<String>,
}

/// Media item for embedding (used in Phase 3: Advanced Features)
#[allow(dead_code)]
struct MediaItem {
    /// Original path
    path: String,

    /// Embedded name (e.g., "image1.png")
    embedded_name: String,

    /// Content type (e.g., "image/png")
    content_type: String,

    /// Raw bytes
    data: Vec<u8>,
}

impl Default for PptxWriter {
    fn default() -> Self {
        Self::new(SlideContract::default())
    }
}

impl PptxWriter {
    /// Create a new PPTX writer with a contract
    pub fn new(contract: SlideContract) -> Self {
        let layout_mapping = LayoutMapping::from_contract(&contract);

        Self {
            contract,
            template: None,
            layout_mapping,
            slides: Vec::new(),
            media: Vec::new(),
            title: None,
            author: None,
        }
    }

    /// Set the template
    pub fn with_template(mut self, template: PotxTemplate) -> Self {
        self.layout_mapping = template.to_layout_mapping();
        self.template = Some(template);
        self
    }

    /// Set the presentation title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add a slide
    pub fn add_slide(&mut self, slide: Slide) {
        self.slides.push(slide);
    }

    /// Add multiple slides
    pub fn add_slides(&mut self, slides: impl IntoIterator<Item = Slide>) {
        self.slides.extend(slides);
    }

    /// Generate the PPTX as bytes
    pub fn generate(&self) -> Result<Vec<u8>> {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = ZipWriter::new(cursor);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Write [Content_Types].xml
        self.write_content_types(&mut zip, options)?;

        // Write _rels/.rels
        self.write_root_rels(&mut zip, options)?;

        // Write docProps/app.xml
        self.write_app_xml(&mut zip, options)?;

        // Write docProps/core.xml
        self.write_core_xml(&mut zip, options)?;

        // Write ppt/presentation.xml
        self.write_presentation_xml(&mut zip, options)?;

        // Write ppt/_rels/presentation.xml.rels
        self.write_presentation_rels(&mut zip, options)?;

        // Write ppt/presProps.xml
        self.write_pres_props(&mut zip, options)?;

        // Write ppt/tableStyles.xml
        self.write_table_styles(&mut zip, options)?;

        // Write ppt/viewProps.xml
        self.write_view_props(&mut zip, options)?;

        // Write minimal theme
        self.write_theme(&mut zip, options)?;

        // Write slide master
        self.write_slide_master(&mut zip, options)?;

        // Write slide layouts
        self.write_slide_layouts(&mut zip, options)?;

        // Write slides
        for (i, slide) in self.slides.iter().enumerate() {
            self.write_slide(&mut zip, options, i + 1, slide)?;

            // Write speaker notes if present
            if slide.notes.is_some() {
                self.write_notes_slide(&mut zip, options, i + 1, slide)?;
            }
        }

        // Write media files
        for media in &self.media {
            let path = format!("ppt/media/{}", media.embedded_name);
            zip.start_file(&path, options)?;
            zip.write_all(&media.data)?;
        }

        let cursor = zip.finish()?;
        Ok(cursor.into_inner())
    }

    /// Write [Content_Types].xml
    fn write_content_types<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("[Content_Types].xml", options)?;

        let mut content = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="jpeg" ContentType="image/jpeg"/>
  <Default Extension="jpg" ContentType="image/jpeg"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/>
  <Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
  <Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
"#,
        );

        // Add slide overrides
        for i in 1..=self.slides.len() {
            content.push_str(&format!(
                "  <Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n",
                i
            ));

            // Add notes override if slide has notes
            if self.slides[i - 1].notes.is_some() {
                content.push_str(&format!(
                    "  <Override PartName=\"/ppt/notesSlides/notesSlide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>\n",
                    i
                ));
            }
        }

        content.push_str("</Types>");

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write _rels/.rels
    fn write_root_rels<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("_rels/.rels", options)?;

        let content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write docProps/app.xml
    fn write_app_xml<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("docProps/app.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <TotalTime>0</TotalTime>
  <Words>0</Words>
  <Application>utf8dok</Application>
  <PresentationFormat>On-screen Show (4:3)</PresentationFormat>
  <Paragraphs>0</Paragraphs>
  <Slides>{}</Slides>
  <Notes>0</Notes>
  <HiddenSlides>0</HiddenSlides>
  <MMClips>0</MMClips>
  <ScaleCrop>false</ScaleCrop>
  <LinksUpToDate>false</LinksUpToDate>
  <SharedDoc>false</SharedDoc>
  <HyperlinksChanged>false</HyperlinksChanged>
  <AppVersion>1.0</AppVersion>
</Properties>"#,
            self.slides.len()
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write docProps/core.xml
    fn write_core_xml<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("docProps/core.xml", options)?;

        let title = self.title.as_deref().unwrap_or("Presentation");
        let author = self.author.as_deref().unwrap_or("utf8dok");
        let now = chrono_lite();

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>{}</dc:title>
  <dc:creator>{}</dc:creator>
  <cp:lastModifiedBy>{}</cp:lastModifiedBy>
  <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
  <dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>
</cp:coreProperties>"#,
            escape_xml(title),
            escape_xml(author),
            escape_xml(author),
            now,
            now
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/presentation.xml
    fn write_presentation_xml<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/presentation.xml", options)?;

        let slide_size = self
            .template
            .as_ref()
            .map(|t| t.slide_size)
            .unwrap_or((DEFAULT_SLIDE_WIDTH_EMU, DEFAULT_SLIDE_HEIGHT_EMU));

        let mut slide_refs = String::new();
        for i in 1..=self.slides.len() {
            slide_refs.push_str(&format!(
                "    <p:sldId id=\"{}\" r:id=\"rId{}\"/>\n",
                255 + i,
                i + 3 // rId1=slideMaster, rId2=presProps, rId3=theme, rId4+=slides
            ));
        }

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="{}" xmlns:r="{}" xmlns:p="{}" saveSubsetFonts="1">
  <p:sldMasterIdLst>
    <p:sldMasterId id="2147483648" r:id="rId1"/>
  </p:sldMasterIdLst>
  <p:sldIdLst>
{}  </p:sldIdLst>
  <p:sldSz cx="{}" cy="{}"/>
  <p:notesSz cx="{}" cy="{}"/>
</p:presentation>"#,
            NS_DRAWING,
            NS_RELATIONSHIPS,
            NS_PRESENTATION,
            slide_refs,
            slide_size.0,
            slide_size.1,
            slide_size.1, // Notes are rotated
            slide_size.0
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/_rels/presentation.xml.rels
    fn write_presentation_rels<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/_rels/presentation.xml.rels", options)?;

        let mut rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps" Target="presProps.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
"#,
        );

        // Add slide relationships
        for i in 1..=self.slides.len() {
            rels.push_str(&format!(
                "  <Relationship Id=\"rId{}\" Type=\"{}\" Target=\"slides/slide{}.xml\"/>\n",
                i + 3,
                REL_TYPE_SLIDE,
                i
            ));
        }

        rels.push_str("</Relationships>");

        zip.write_all(rels.as_bytes())?;
        Ok(())
    }

    /// Write ppt/presProps.xml
    fn write_pres_props<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/presProps.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentationPr xmlns:a="{}" xmlns:r="{}" xmlns:p="{}">
  <p:extLst/>
</p:presentationPr>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/tableStyles.xml
    fn write_table_styles<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/tableStyles.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:tblStyleLst xmlns:a="{}" def="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}"/>"#,
            NS_DRAWING
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/viewProps.xml
    fn write_view_props<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/viewProps.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:viewPr xmlns:a="{}" xmlns:r="{}" xmlns:p="{}">
  <p:normalViewPr>
    <p:restoredLeft sz="15620"/>
    <p:restoredTop sz="94660"/>
  </p:normalViewPr>
  <p:slideViewPr>
    <p:cSldViewPr>
      <p:cViewPr>
        <p:scale>
          <a:sx n="100" d="100"/>
          <a:sy n="100" d="100"/>
        </p:scale>
        <p:origin x="0" y="0"/>
      </p:cViewPr>
    </p:cSldViewPr>
  </p:slideViewPr>
</p:viewPr>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/theme/theme1.xml
    fn write_theme<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/theme/theme1.xml", options)?;

        let theme_name = self
            .template
            .as_ref()
            .and_then(|t| t.theme.as_ref())
            .map(|t| t.name.as_str())
            .unwrap_or("utf8dok");

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="{}" name="{}">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont>
        <a:latin typeface="Calibri Light"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Calibri"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#,
            NS_DRAWING, theme_name
        );

        zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write ppt/slideMasters/slideMaster1.xml
    fn write_slide_master<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file("ppt/slideMasters/slideMaster1.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="{}" xmlns:r="{}" xmlns:p="{}">
  <p:cSld>
    <p:bg>
      <p:bgRef idx="1001">
        <a:schemeClr val="bg1"/>
      </p:bgRef>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst>
    <p:sldLayoutId id="2147483649" r:id="rId1"/>
    <p:sldLayoutId id="2147483650" r:id="rId2"/>
  </p:sldLayoutIdLst>
</p:sldMaster>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION
        );

        zip.write_all(content.as_bytes())?;

        // Write slide master rels
        zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;

        let rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{}">
  <Relationship Id="rId1" Type="{}" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="{}" Target="../slideLayouts/slideLayout2.xml"/>
  <Relationship Id="rId3" Type="{}" Target="../theme/theme1.xml"/>
</Relationships>"#,
            NS_RELATIONSHIPS, REL_TYPE_SLIDE_LAYOUT, REL_TYPE_SLIDE_LAYOUT, REL_TYPE_THEME
        );

        zip.write_all(rels.as_bytes())?;
        Ok(())
    }

    /// Write ppt/slideLayouts/slideLayoutN.xml
    fn write_slide_layouts<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
    ) -> Result<()> {
        // Layout 1: Title Slide
        zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="{}" xmlns:r="{}" xmlns:p="{}" type="title" preserve="1">
  <p:cSld name="Title Slide">
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title 1"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="ctrTitle"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="685800" y="2130425"/>
            <a:ext cx="7772400" cy="1470025"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p><a:endParaRPr lang="en-US"/></a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Subtitle 2"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="1371600" y="3886200"/>
            <a:ext cx="6400800" cy="1752600"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p><a:endParaRPr lang="en-US"/></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION
        );

        zip.write_all(content.as_bytes())?;

        // Layout 1 rels
        zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)?;
        let rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{}">
  <Relationship Id="rId1" Type="{}" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#,
            NS_RELATIONSHIPS, REL_TYPE_SLIDE_MASTER
        );
        zip.write_all(rels.as_bytes())?;

        // Layout 2: Title and Content
        zip.start_file("ppt/slideLayouts/slideLayout2.xml", options)?;

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="{}" xmlns:r="{}" xmlns:p="{}" type="obj" preserve="1">
  <p:cSld name="Title and Content">
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title 1"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="title"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="457200" y="274638"/>
            <a:ext cx="8229600" cy="1143000"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p><a:endParaRPr lang="en-US"/></a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Content Placeholder 2"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="457200" y="1600200"/>
            <a:ext cx="8229600" cy="4525963"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p><a:endParaRPr lang="en-US"/></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION
        );

        zip.write_all(content.as_bytes())?;

        // Layout 2 rels
        zip.start_file("ppt/slideLayouts/_rels/slideLayout2.xml.rels", options)?;
        zip.write_all(rels.as_bytes())?;

        Ok(())
    }

    /// Write a single slide
    fn write_slide<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
        slide_num: usize,
        slide: &Slide,
    ) -> Result<()> {
        zip.start_file(format!("ppt/slides/slide{}.xml", slide_num), options)?;

        let layout_idx = self.layout_mapping.get_layout_for_hint(slide.layout_hint);

        let content = self.generate_slide_xml(slide, layout_idx)?;
        zip.write_all(content.as_bytes())?;

        // Write slide rels
        zip.start_file(
            format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
            options,
        )?;

        let mut rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{}">
  <Relationship Id="rId1" Type="{}" Target="../slideLayouts/slideLayout{}.xml"/>
"#,
            NS_RELATIONSHIPS, REL_TYPE_SLIDE_LAYOUT, layout_idx
        );

        // Add notes relationship if present
        if slide.notes.is_some() {
            rels.push_str(&format!(
                "  <Relationship Id=\"rId2\" Type=\"{}\" Target=\"../notesSlides/notesSlide{}.xml\"/>\n",
                REL_TYPE_NOTES_SLIDE,
                slide_num
            ));
        }

        rels.push_str("</Relationships>");

        zip.write_all(rels.as_bytes())?;
        Ok(())
    }

    /// Generate slide XML content
    fn generate_slide_xml(&self, slide: &Slide, _layout_idx: u32) -> Result<String> {
        let mut shapes = String::new();

        // Add title shape if present
        if let Some(title) = &slide.title {
            shapes.push_str(&self.generate_title_shape(title, slide.is_title_slide()));
        }

        // Add subtitle for title slides
        if slide.is_title_slide() {
            if let Some(subtitle) = &slide.subtitle {
                shapes.push_str(&self.generate_subtitle_shape(subtitle));
            }
        }

        // Add content shapes
        if !slide.content.is_empty() {
            shapes.push_str(&self.generate_content_shapes(&slide.content)?);
        }

        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="{}" xmlns:r="{}" xmlns:p="{}">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
{}    </p:spTree>
  </p:cSld>
</p:sld>"#,
            NS_DRAWING, NS_RELATIONSHIPS, NS_PRESENTATION, shapes
        );

        Ok(xml)
    }

    /// Generate title shape XML
    fn generate_title_shape(&self, title: &str, is_title_slide: bool) -> String {
        let ph_type = if is_title_slide { "ctrTitle" } else { "title" };

        format!(
            r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title 1"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="{}"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="{}"/>
              <a:t>{}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
"#,
            ph_type,
            self.contract.meta.locale,
            escape_xml(title)
        )
    }

    /// Generate subtitle shape XML
    fn generate_subtitle_shape(&self, subtitle: &str) -> String {
        format!(
            r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Subtitle 2"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="{}"/>
              <a:t>{}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
"#,
            self.contract.meta.locale,
            escape_xml(subtitle)
        )
    }

    /// Generate content shapes from SlideContent
    fn generate_content_shapes(&self, content: &[SlideContent]) -> Result<String> {
        let mut shapes = String::new();
        let mut shape_id = 4; // After title and subtitle

        for item in content {
            match item {
                SlideContent::Paragraph(text) => {
                    shapes.push_str(&self.generate_text_shape(shape_id, text));
                    shape_id += 1;
                }
                SlideContent::BulletList(list) => {
                    shapes.push_str(&self.generate_bullet_list_shape(shape_id, list));
                    shape_id += 1;
                }
                SlideContent::NumberedList(list) => {
                    shapes.push_str(&self.generate_numbered_list_shape(shape_id, list));
                    shape_id += 1;
                }
                // Other content types will be implemented in later phases
                _ => {}
            }
        }

        Ok(shapes)
    }

    /// Generate a text paragraph shape
    fn generate_text_shape(&self, id: u32, text: &TextContent) -> String {
        format!(
            r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{}" name="Content {}"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
{}          </a:p>
        </p:txBody>
      </p:sp>
"#,
            id,
            id,
            self.generate_text_runs(&text.runs)
        )
    }

    /// Generate bullet list shape
    fn generate_bullet_list_shape(&self, id: u32, list: &ListContent) -> String {
        let mut paragraphs = String::new();

        for item in &list.items {
            paragraphs.push_str(&self.generate_list_item_paragraph(item, false));
        }

        format!(
            r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{}" name="Content {}"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
{}        </p:txBody>
      </p:sp>
"#,
            id, id, paragraphs
        )
    }

    /// Generate numbered list shape
    fn generate_numbered_list_shape(&self, id: u32, list: &ListContent) -> String {
        let mut paragraphs = String::new();

        for item in &list.items {
            paragraphs.push_str(&self.generate_list_item_paragraph(item, true));
        }

        format!(
            r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{}" name="Content {}"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
{}        </p:txBody>
      </p:sp>
"#,
            id, id, paragraphs
        )
    }

    /// Generate a list item as a paragraph
    fn generate_list_item_paragraph(&self, item: &ListItem, _numbered: bool) -> String {
        format!(
            r#"          <a:p>
            <a:pPr lvl="{}"/>
{}          </a:p>
"#,
            item.level,
            self.generate_text_runs(&item.content.runs)
        )
    }

    /// Generate text runs
    fn generate_text_runs(&self, runs: &[TextRun]) -> String {
        let mut result = String::new();

        for run in runs {
            let mut rpr = format!("lang=\"{}\"", self.contract.meta.locale);

            if run.bold {
                rpr.push_str(" b=\"1\"");
            }
            if run.italic {
                rpr.push_str(" i=\"1\"");
            }

            result.push_str(&format!(
                "            <a:r>\n              <a:rPr {}/>\n              <a:t>{}</a:t>\n            </a:r>\n",
                rpr,
                escape_xml(&run.text)
            ));
        }

        result
    }

    /// Write speaker notes slide
    fn write_notes_slide<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        options: SimpleFileOptions,
        slide_num: usize,
        slide: &Slide,
    ) -> Result<()> {
        let notes = slide.notes.as_ref().unwrap();

        zip.start_file(
            format!("ppt/notesSlides/notesSlide{}.xml", slide_num),
            options,
        )?;

        let notes_text = notes.as_plain_text();

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="{}" xmlns:r="{}" xmlns:p="{}">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Slide Image Placeholder 1"/>
          <p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="sldImg"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Notes Placeholder 2"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="{}"/>
              <a:t>{}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:notes>"#,
            NS_DRAWING,
            NS_RELATIONSHIPS,
            NS_PRESENTATION,
            self.contract.meta.locale,
            escape_xml(&notes_text)
        );

        zip.write_all(content.as_bytes())?;

        // Write notes slide rels
        zip.start_file(
            format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", slide_num),
            options,
        )?;

        let rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{}">
  <Relationship Id="rId1" Type="{}" Target="../slides/slide{}.xml"/>
</Relationships>"#,
            NS_RELATIONSHIPS, REL_TYPE_SLIDE, slide_num
        );

        zip.write_all(rels.as_bytes())?;
        Ok(())
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Get a simple ISO-8601 timestamp (without chrono dependency)
fn chrono_lite() -> String {
    // Simple timestamp - in production we'd use chrono
    "2025-01-01T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slide::{SlideLayoutHint, SpeakerNotes};
    use zip::ZipArchive;

    #[test]
    fn test_create_writer() {
        let writer = PptxWriter::default();
        assert!(writer.slides.is_empty());
    }

    #[test]
    fn test_add_slides() {
        let mut writer = PptxWriter::default();

        writer.add_slide(Slide::title_slide(1, "Hello", Some("World".to_string())));
        writer.add_slide(Slide::content_slide(2, "Content"));

        assert_eq!(writer.slides.len(), 2);
    }

    #[test]
    fn test_generate_empty_pptx() {
        let writer = PptxWriter::default()
            .with_title("Test Presentation")
            .with_author("Test Author");

        let result = writer.generate();
        assert!(result.is_ok());

        let bytes = result.unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP
        let cursor = Cursor::new(bytes);
        let archive = ZipArchive::new(cursor);
        assert!(archive.is_ok());
    }

    #[test]
    fn test_generate_with_slides() {
        let mut writer = PptxWriter::default()
            .with_title("Test Presentation");

        writer.add_slide(Slide::title_slide(1, "Welcome", Some("To the presentation".to_string())));
        writer.add_slide(
            Slide::content_slide(2, "Overview")
                .with_content(SlideContent::BulletList(ListContent {
                    items: vec![
                        ListItem::simple("First point"),
                        ListItem::simple("Second point"),
                    ],
                })),
        );

        let result = writer.generate();
        assert!(result.is_ok());

        let bytes = result.unwrap();
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();

        // Verify slides exist
        assert!(archive.by_name("ppt/slides/slide1.xml").is_ok());
        assert!(archive.by_name("ppt/slides/slide2.xml").is_ok());
    }

    #[test]
    fn test_generate_with_speaker_notes() {
        let mut writer = PptxWriter::default();

        writer.add_slide(
            Slide::content_slide(1, "With Notes")
                .with_notes(SpeakerNotes::from_text("These are my speaker notes")),
        );

        let result = writer.generate();
        assert!(result.is_ok());

        let bytes = result.unwrap();
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();

        // Verify notes slide exists
        assert!(archive.by_name("ppt/notesSlides/notesSlide1.xml").is_ok());
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello & World"), "Hello &amp; World");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_with_template() {
        let template = PotxTemplate::minimal();
        let writer = PptxWriter::default().with_template(template);

        assert!(writer.template.is_some());
    }

    #[test]
    fn test_layout_selection() {
        let writer = PptxWriter::default();

        let idx = writer.layout_mapping.get_layout_for_hint(SlideLayoutHint::Title);
        assert_eq!(idx, 1);

        let idx = writer.layout_mapping.get_layout_for_hint(SlideLayoutHint::Content);
        assert_eq!(idx, 2);
    }
}
