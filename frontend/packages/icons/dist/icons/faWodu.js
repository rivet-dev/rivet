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

// src/node_modules/@fortawesome/free-brands-svg-icons/faWodu.js
var require_faWodu = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faWodu.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "wodu";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e088";
    var svgPathData = "M178.4 339.7l-37.3 0-28.9-116.2-.5 0-28.5 116.2-38 0-45.2-170.8 37.5 0 27 116.2 .5 0 29.7-116.2 35.2 0 29.2 117.7 .5 0 28-117.7 36.8 0-45.9 170.8zm93-127c39 0 64.1 25.8 64.1 65.3 0 39.2-25.1 65-64.1 65-38.7 0-63.9-25.8-63.9-65 0-39.5 25.1-65.3 63.9-65.3zm0 104.8c23.2 0 30.1-19.9 30.1-39.5 0-19.9-6.9-39.7-30.1-39.7-27.7 0-29.9 19.9-29.9 39.7 0 19.6 6.9 39.5 29.9 39.5zm163.7 6.5l-.5 0c-7.9 13.4-21.8 19.1-37.5 19.1-37.3 0-55.5-32-55.5-66.2 0-33.2 18.4-64.1 54.8-64.1 14.6 0 28.9 6.2 36.8 18.4l.2 0 0-62.2 34 0 0 170.8-32.3 0 0-15.8zm-29.7-85.6c-22.2 0-29.9 19.1-29.9 39.5 0 19.4 8.8 39.7 29.9 39.7 22.5 0 29.2-19.6 29.2-39.9 0-20.1-7.2-39.2-29.2-39.2zM593 339.7l-32.3 0 0-17.2-.7 0c-8.6 13.9-23.4 20.6-37.8 20.6-36.1 0-45.2-20.3-45.2-50.9l0-76.1 34 0 0 69.8c0 20.3 6 30.4 21.8 30.4 18.4 0 26.3-10.3 26.3-35.4l0-64.8 34 0 0 123.6zm9.5-36.8l37.5 0 0 36.8-37.5 0 0-36.8z";
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
    exports.faWodu = exports.definition;
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

// src/_entries/faWodu.js
var import_faWodu = __toESM(require_faWodu());
var export_faWodu = import_faWodu.definition;
export {
  export_faWodu as faWodu
};
