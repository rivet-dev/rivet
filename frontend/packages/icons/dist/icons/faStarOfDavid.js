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

// src/node_modules/@fortawesome/free-solid-svg-icons/faStarOfDavid.js
var require_faStarOfDavid = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faStarOfDavid.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "star-of-david";
    var width = 512;
    var height = 512;
    var aliases = [10017];
    var unicode = "f69a";
    var svgPathData = "M415.9 311.9l-27.8 48.1 55.7 0-27.8-48.1zM383.6 256l-60.2-104-134.8 0-60.2 104 60.2 104 134.8 0 60.2-104zm64.7 0l58.4 100.9c3.5 6 5.3 12.8 5.3 19.7 0 21.7-17.6 39.4-39.4 39.4l-117 0-61.3 105.8C286.5 535.6 271.8 544 256 544s-30.5-8.4-38.4-22.2l-61.3-105.8-117 0c-21.7 0-39.4-17.6-39.4-39.4 0-6.9 1.8-13.7 5.3-19.7L63.7 256 5.3 155.1C1.8 149.1 0 142.3 0 135.4 0 113.6 17.6 96 39.4 96l117 0 61.3-105.8C225.5-23.6 240.2-32 256-32s30.5 8.4 38.4 22.2l61.3 105.8 117 0c21.7 0 39.4 17.6 39.4 39.4 0 6.9-1.8 13.7-5.3 19.7L448.3 256zM388.1 152l27.8 48.1 27.8-48.1-55.7 0zM291 96l-35-60.4-35 60.4 70 0zM123.9 152l-55.7 0 27.8 48.1 27.8-48.1zM96.1 311.9l-27.8 48.1 55.7 0-27.8-48.1zM221 416l35 60.4 35-60.4-70 0z";
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
    exports.faStarOfDavid = exports.definition;
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

// src/_entries/faStarOfDavid.js
var import_faStarOfDavid = __toESM(require_faStarOfDavid());
var export_faStarOfDavid = import_faStarOfDavid.definition;
export {
  export_faStarOfDavid as faStarOfDavid
};
