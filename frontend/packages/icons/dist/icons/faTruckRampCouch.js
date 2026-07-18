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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckRampCouch.js
var require_faTruckRampCouch = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckRampCouch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-ramp-couch";
    var width = 640;
    var height = 512;
    var aliases = ["truck-couch"];
    var unicode = "f4dd";
    var svgPathData = "M640 0l0 400c0 61.9-50.1 112-112 112c-61 0-110.5-48.7-112-109.3L48.4 502.9c-17.1 4.6-34.6-5.4-39.3-22.5s5.4-34.6 22.5-39.3L352 353.8 352 64c0-35.3 28.7-64 64-64L640 0zM576 400a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM294.5 209.2c8.7-2.3 17.4-2.1 25.5 .2l0 125.5-38.7 10.4L126.7 386.6 64.9 403.2c-17.1 4.6-34.6-5.6-39.2-22.6L13.3 334.2c-6.9-25.6 8.3-51.9 33.9-58.8s51.9 8.3 58.8 33.9l4.1 15.5 154.5-41.4-4.1-15.5c-6.9-25.6 8.3-51.9 33.9-58.8zM8 252.8c-9.1-34.1 11.1-69.2 45.3-78.4l185.5-49.7c34.1-9.1 69.2 11.1 78.4 45.3l1.7 6.4c-10.6-1.6-21.6-1.1-32.6 1.9c-37.4 10-61.5 44.9-59.2 82.1L131.2 286c-16.6-33.4-54.9-51.5-92.3-41.5c-11 2.9-20.8 8-29.2 14.7L8 252.8z";
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
    exports.faTruckRampCouch = exports.definition;
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

// src/_entries/faTruckRampCouch.js
var import_faTruckRampCouch = __toESM(require_faTruckRampCouch());
var export_faTruckRampCouch = import_faTruckRampCouch.definition;
export {
  export_faTruckRampCouch as faTruckRampCouch
};
