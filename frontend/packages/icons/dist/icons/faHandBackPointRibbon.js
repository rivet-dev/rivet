var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointRibbon.js
var require_faHandBackPointRibbon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointRibbon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-back-point-ribbon";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e1a0";
    var svgPathData = "M112 128l-68.2 0c-6.5 0-11.8-5.3-11.8-11.8c0-3.9 1.9-7.5 5.1-9.7L75.6 80 37.1 53.5c-3.2-2.2-5.1-5.8-5.1-9.7C32 37.3 37.3 32 43.8 32l69 0C116.5 13.7 132.6 0 152 0c22.1 0 40 17.9 40 40l0 8 0 64 0 16-16 0-64 0zm0 112l0-80 80 0 0 28.2c8.5-7.6 19.7-12.2 32-12.2c25.3 0 46 19.5 47.9 44.3c8.5-7.7 19.8-12.3 32.1-12.3c25.3 0 46 19.5 47.9 44.3c8.5-7.7 19.8-12.3 32.1-12.3c26.5 0 48 21.5 48 48l0 32 0 64c0 70.7-57.3 128-128 128l-118.1 0c-37.4 0-73.7-13.1-102.4-37.1l-5.4-4.5C48.9 446.1 32 410 32 372L32 334c0-19 8.4-37 23-49.2l19.2-16c1.9-1.6 3.8-3.1 5.8-4.5L80 320c0 8.8 7.2 16 16 16s16-7.2 16-16l0-73 0-7z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faHandBackPointRibbon = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faHandBackPointRibbon.js
var import_faHandBackPointRibbon = __toESM(require_faHandBackPointRibbon());
var export_faHandBackPointRibbon = import_faHandBackPointRibbon.definition;
export {
  export_faHandBackPointRibbon as faHandBackPointRibbon
};
