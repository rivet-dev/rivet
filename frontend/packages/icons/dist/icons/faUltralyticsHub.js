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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUltralyticsHub.js
var require_faUltralyticsHub = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUltralyticsHub.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ultralytics-hub";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e86e";
    var svgPathData = "M130.4 16c60.9 0 110.4 49.5 110.4 110.4l0 24.9c4.8-.7 9.7-1 14.6-1 88.1-.2 165.1 49 205.4 121.2-22.8-16.2-50.7-25.4-80.4-25.3-77.1 .2-139.5 62.6-139.6 139.4-.1 60.9-49.5 110.3-110.7 110.5-60.6 .2-110.2-49.5-110.2-110.5 .2-2.6 0 0 .1-3.2l0-256C20.1 65.5 69.5 16 130.4 16zM380.3 496a110.4 110.4 0 1 1 0-220.9 110.4 110.4 0 1 1 0 220.9z";
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
    exports.faUltralyticsHub = exports.definition;
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

// src/_entries/faUltralyticsHub.js
var import_faUltralyticsHub = __toESM(require_faUltralyticsHub());
var export_faUltralyticsHub = import_faUltralyticsHub.definition;
export {
  export_faUltralyticsHub as faUltralyticsHub
};
