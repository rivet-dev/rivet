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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClouds.js
var require_faClouds = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClouds.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clouds";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f744";
    var svgPathData = "M304 160c-74.1 0-135.2 56-143.1 128L96 288c-53 0-96-43-96-96s43-96 96-96l1.1 0C104.9 41.7 151.6 0 208 0c47.9 0 88.8 30.1 104.8 72.4C324.8 67 338 64 352 64c53 0 96 43 96 96c0 12.1-2.2 23.6-6.3 34.2c-11.4 2.3-22.2 6.4-32.1 11.9C383.3 177.7 345.7 160 304 160zM224 512c-53 0-96-43-96-96c0-42.5 27.6-78.6 65.9-91.2c-1.3-6.7-1.9-13.7-1.9-20.8c0-61.9 50.1-112 112-112c43.1 0 80.5 24.3 99.2 60c14.7-17.1 36.5-28 60.8-28c44.2 0 80 35.8 80 80c0 5.5-.6 10.8-1.6 16c.5 0 1.1 0 1.6 0c53 0 96 43 96 96s-43 96-96 96l-320 0z";
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
    exports.faClouds = exports.definition;
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

// src/_entries/faClouds.js
var import_faClouds = __toESM(require_faClouds());
var export_faClouds = import_faClouds.definition;
export {
  export_faClouds as faClouds
};
