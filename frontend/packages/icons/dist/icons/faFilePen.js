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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFilePen.js
var require_faFilePen = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFilePen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-pen";
    var width = 640;
    var height = 512;
    var aliases = [128221, "file-edit"];
    var unicode = "f31c";
    var svgPathData = "M128.1 0c-35.3 0-64 28.7-64 64l0 384c0 35.3 28.7 64 64 64l146.2 0 10.9-54.5c4.3-21.7 15-41.6 30.6-57.2l132.2-132.2 0-97.5c0-17-6.7-33.3-18.7-45.3L322.8 18.7C310.8 6.7 294.5 0 277.6 0L128.1 0zM389.6 176l-93.5 0c-13.3 0-24-10.7-24-24l0-93.5 117.5 117.5zM332.3 466.9l-11.9 59.6c-.2 .9-.3 1.9-.3 2.9 0 8 6.5 14.6 14.6 14.6 1 0 1.9-.1 2.9-.3l59.6-11.9c12.4-2.5 23.8-8.6 32.7-17.5l118.9-118.9-80-80-118.9 118.9c-8.9 8.9-15 20.3-17.5 32.7zm267.8-123c22.1-22.1 22.1-57.9 0-80s-57.9-22.1-80 0l-28.8 28.8 80 80 28.8-28.8z";
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
    exports.faFilePen = exports.definition;
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

// src/_entries/faFilePen.js
var import_faFilePen = __toESM(require_faFilePen());
var export_faFilePen = import_faFilePen.definition;
export {
  export_faFilePen as faFilePen
};
