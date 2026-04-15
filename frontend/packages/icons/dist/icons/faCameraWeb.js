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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraWeb.js
var require_faCameraWeb = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraWeb.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-web";
    var width = 448;
    var height = 512;
    var aliases = ["webcam"];
    var unicode = "f832";
    var svgPathData = "M344.2 413.1C406.6 373.3 448 303.5 448 224C448 100.3 347.7 0 224 0S0 100.3 0 224c0 79.5 41.4 149.3 103.8 189.1L45.6 453.8c-11.4 8-16.4 22.5-12.2 35.8S50 512 64 512l320 0c14 0 26.3-9.1 30.5-22.4s-.7-27.8-12.2-35.8l-58.2-40.7zM224 352a128 128 0 1 0 0-256 128 128 0 1 0 0 256zm0-288a160 160 0 1 1 0 320 160 160 0 1 1 0-320zM176 224c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-44.2 35.8-80 80-80c8.8 0 16 7.2 16 16s-7.2 16-16 16c-26.5 0-48 21.5-48 48z";
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
    exports.faCameraWeb = exports.definition;
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

// src/_entries/faCameraWeb.js
var import_faCameraWeb = __toESM(require_faCameraWeb());
var export_faCameraWeb = import_faCameraWeb.definition;
export {
  export_faCameraWeb as faCameraWeb
};
