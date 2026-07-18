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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileSvg.js
var require_faFileSvg = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileSvg.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-svg";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e64b";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-208 0c-35.3 0-64 28.7-64 64l0 144-48 0c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM205.7 352l26.3 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-26.3 0c-7.5 0-13.7 6.1-13.7 13.7c0 5.2 2.9 9.9 7.6 12.2l31.2 15.6c15.5 7.7 25.2 23.5 25.2 40.8c0 25.2-20.4 45.7-45.7 45.7L176 512c-8.8 0-16-7.2-16-16s7.2-16 16-16l34.3 0c7.5 0 13.7-6.1 13.7-13.7c0-5.2-2.9-9.9-7.6-12.2l-31.2-15.6C169.8 430.8 160 415 160 397.7c0-25.2 20.4-45.7 45.7-45.7zM456 352l16 0c22.1 0 40 17.9 40 40l0 8c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-8c0-4.4-3.6-8-8-8l-16 0c-4.4 0-8 3.6-8 8l0 80c0 4.4 3.6 8 8 8l16 0c4.4 0 8-3.6 8-8l0-8c-8.8 0-16-7.2-16-16s7.2-16 16-16l16 0c8.8 0 16 7.2 16 16l0 24c0 22.1-17.9 40-40 40l-16 0c-22.1 0-40-17.9-40-40l0-80c0-22.1 17.9-40 40-40zm-152 0c8.8 0 16 7.2 16 16l0 31.6c0 23 5.5 45.6 16 66c10.5-20.3 16-42.9 16-66l0-31.6c0-8.8 7.2-16 16-16s16 7.2 16 16l0 31.6c0 34.7-10.3 68.7-29.6 97.6l-5.1 7.7c-3 4.5-8 7.1-13.3 7.1s-10.3-2.7-13.3-7.1l-5.1-7.7c-19.3-28.9-29.6-62.9-29.6-97.6l0-31.6c0-8.8 7.2-16 16-16z";
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
    exports.faFileSvg = exports.definition;
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

// src/_entries/faFileSvg.js
var import_faFileSvg = __toESM(require_faFileSvg());
var export_faFileSvg = import_faFileSvg.definition;
export {
  export_faFileSvg as faFileSvg
};
