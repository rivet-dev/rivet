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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUltralyticsYolo.js
var require_faUltralyticsYolo = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUltralyticsYolo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ultralytics-yolo";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e86f";
    var svgPathData = "M383.5 10.8c61.8-.2 112.3 50.6 112.3 112.9-.2 2.7 0 0-.1 3.3l.3 .3c-1.5 89.9-53 168.4-127.4 208.5l0 52.8c0 62.6-51.1 113.5-113.7 112.9-62-.6-111.4-52.2-111.4-114.3l0-51.2c-40.7-21.9-74.5-55.2-97-95.6 23.2 16.5 51.7 26 82 25.9 78.6-.2 142.2-64 142.3-142.5 .1-62.3 50.5-112.7 112.9-112.9zm-255 225.8a112.9 112.9 0 1 1 0-225.8 112.9 112.9 0 1 1 0 225.8z";
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
    exports.faUltralyticsYolo = exports.definition;
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

// src/_entries/faUltralyticsYolo.js
var import_faUltralyticsYolo = __toESM(require_faUltralyticsYolo());
var export_faUltralyticsYolo = import_faUltralyticsYolo.definition;
export {
  export_faUltralyticsYolo as faUltralyticsYolo
};
