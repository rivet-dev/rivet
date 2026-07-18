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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAnxiousSweat.js
var require_faFaceAnxiousSweat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAnxiousSweat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-anxious-sweat";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e36a";
    var svgPathData = "M288 512c141.4 0 256-114.6 256-256S429.4 0 288 0S32 114.6 32 256c0 1.1 0 2.2 0 3.3c16.3-7.1 36.4-2.8 48 12.8c7 9.2 18.1 24.6 27.7 40.7c4.8 8.1 9.6 17 13.2 25.7c3.4 8 7.1 19 7.1 30.5c0 24.5-11.1 46-28.4 60.3C146.4 480.2 213.5 512 288 512zM79.5 404.5C89.6 395.9 96 383.2 96 369c0-20-28.4-60.4-41.6-77.7c-3.2-4.4-9.6-4.4-12.8 0c-1.7 2.3-3.8 5-5.9 8.1C21.4 319.1 0 351.7 0 369c0 26 21.5 47 48 47c12 0 23-4.3 31.5-11.5zM208 400c0-44.2 35.8-80 80-80s80 35.8 80 80c0 8.8-7.2 16-16 16l-128 0c-8.8 0-16-7.2-16-16zM367.6 240a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm-128 32a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zM227.3 171.3l-7.4 7.4c-23.6 23.6-53.9 39.2-86.8 44.7l-2.5 .4c-8.7 1.5-17-4.4-18.4-13.2s4.4-17 13.2-18.4l2.5-.4c26.3-4.4 50.6-16.9 69.5-35.7l7.4-7.4c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6zm121.4-22.6c6.2-6.2 16.4-6.2 22.6 0l7.4 7.4c18.9 18.9 43.2 31.4 69.5 35.7l2.5 .4c8.7 1.5 14.6 9.7 13.2 18.4s-9.7 14.6-18.4 13.2l-2.5-.4c-32.9-5.5-63.3-21.1-86.8-44.7l-7.4-7.4c-6.2-6.2-6.2-16.4 0-22.6z";
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
    exports.faFaceAnxiousSweat = exports.definition;
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

// src/_entries/faFaceAnxiousSweat.js
var import_faFaceAnxiousSweat = __toESM(require_faFaceAnxiousSweat());
var export_faFaceAnxiousSweat = import_faFaceAnxiousSweat.definition;
export {
  export_faFaceAnxiousSweat as faFaceAnxiousSweat
};
