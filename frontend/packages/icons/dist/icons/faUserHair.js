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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHair.js
var require_faUserHair = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHair.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-hair";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e45a";
    var svgPathData = "M304 128l0 16c0 44.2-35.8 80-80 80s-80-35.8-80-80l0-16c0-5.5 .6-10.8 1.6-16l30.4 0c29.8 0 55.9-16.3 69.6-40.5C257.3 86.4 275.5 96 296 96l1.3 0c4.3 9.8 6.7 20.6 6.7 32zM96 128l0 16c0 70.7 57.3 128 128 128s128-57.3 128-128l0-16C352 57.3 294.7 0 224 0S96 57.3 96 128zm65.3 192C72.2 320 0 392.2 0 481.3c0 17 13.8 30.7 30.7 30.7l386.6 0c17 0 30.7-13.8 30.7-30.7C448 392.2 375.8 320 286.7 320l-125.4 0z";
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
    exports.faUserHair = exports.definition;
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

// src/_entries/faUserHair.js
var import_faUserHair = __toESM(require_faUserHair());
var export_faUserHair = import_faUserHair.definition;
export {
  export_faUserHair as faUserHair
};
