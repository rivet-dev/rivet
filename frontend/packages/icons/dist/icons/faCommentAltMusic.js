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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageMusic.js
var require_faMessageMusic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageMusic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "message-music";
    var width = 512;
    var height = 512;
    var aliases = ["comment-alt-music"];
    var unicode = "f8af";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L448 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64l-138.7 0L185.6 508.8c-4.8 3.6-11.3 4.2-16.8 1.5s-8.8-8.2-8.8-14.3l0-80-96 0c-35.3 0-64-28.7-64-64L0 64zM352 96c0-5.2-2.6-10.2-6.9-13.2s-9.8-3.7-14.7-1.8l-128 48c-6.2 2.3-10.4 8.3-10.4 15l0 32 0 81.8c-5-1.2-10.4-1.8-16-1.8c-26.5 0-48 14.3-48 32s21.5 32 48 32s48-14.3 48-32l0-100.9 96-36 0 74.7c-5-1.2-10.4-1.8-16-1.8c-26.5 0-48 14.3-48 32s21.5 32 48 32s48-14.3 48-32l0-128 0-32z";
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
    exports.faMessageMusic = exports.definition;
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

// src/_entries/faCommentAltMusic.js
var import_faMessageMusic = __toESM(require_faMessageMusic());
var export_faCommentAltMusic = import_faMessageMusic.definition;
export {
  export_faCommentAltMusic as faCommentAltMusic
};
