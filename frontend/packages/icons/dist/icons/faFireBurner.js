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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFireBurner.js
var require_faFireBurner = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFireBurner.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fire-burner";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4f1";
    var svgPathData = "M281.7 48.3L294 34.5c5.4-6.1 13.3-8.8 20.9-8.9 7.2 0 14.3 2.6 19.9 7.8 19.7 18.3 39.8 43.2 55 70.6 15.1 27.2 26.2 58.1 26.2 88.1 0 88.7-71.3 159.8-160 159.8-89.6 0-160-71.3-160-159.8 0-37.3 16-73.4 36.8-104.5 20.9-31.3 47.5-59 70.9-80.2 5.7-5.2 13.1-7.7 20.3-7.5s13.4 3.2 18.8 7.5c14.4 11.4 38.9 40.7 38.9 40.7zM320 240.2c0-36.5-37-73-54.8-88.4-5.4-4.7-13.1-4.7-18.5 0-17.7 15.4-54.8 51.9-54.8 88.4 0 35.3 28.7 64 64 64s64-28.7 64-64zM92.8 320c37.7 48.3 96.1 80 163.2 80 66.5 0 125.1-31.7 163-80l45 0c26.5 0 48 21.5 48 48l0 64c0 26.5-21.5 48-48 48L48 480c-26.5 0-48-21.5-48-48l0-64c0-26.5 21.5-48 48-48l44.8 0zM456 400a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faFireBurner = exports.definition;
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

// src/_entries/faFireBurner.js
var import_faFireBurner = __toESM(require_faFireBurner());
var export_faFireBurner = import_faFireBurner.definition;
export {
  export_faFireBurner as faFireBurner
};
