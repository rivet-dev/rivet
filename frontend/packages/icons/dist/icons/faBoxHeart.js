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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxHeart.js
var require_faBoxHeart = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxHeart.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "box-heart";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f49d";
    var svgPathData = "M93.7 32L208 32l0 128L0 160 50.7 58.5C58.9 42.3 75.5 32 93.7 32zM240 32l114.3 0c18.2 0 34.8 10.3 42.9 26.5L448 160l-208 0 0-128zM0 192l448 0 0 224c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 192zM212.7 420.7c6.2 6.2 16.4 6.2 22.6 0l77.3-77.3c20.5-20.5 20.5-53.8 0-74.3s-53.8-20.5-74.3 0L224 283.4l-14.3-14.3c-20.5-20.5-53.8-20.5-74.3 0s-20.5 53.8 0 74.3l77.3 77.3z";
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
    exports.faBoxHeart = exports.definition;
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

// src/_entries/faBoxHeart.js
var import_faBoxHeart = __toESM(require_faBoxHeart());
var export_faBoxHeart = import_faBoxHeart.definition;
export {
  export_faBoxHeart as faBoxHeart
};
