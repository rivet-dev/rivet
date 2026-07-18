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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFluxCapacitor.js
var require_faFluxCapacitor = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFluxCapacitor.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flux-capacitor";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8ba";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zM148.7 237.3l-49-49c-15.6-15.6-15.6-40.9 0-56.6s40.9-15.6 56.6 0l49 49c1.8-6.6 2.7-13.6 2.7-20.7c0-44.2-35.8-80-80-80s-80 35.8-80 80s35.8 80 80 80c7.2 0 14.1-.9 20.7-2.7zM184 282.7c-23.9 13.8-40 39.7-40 69.3c0 44.2 35.8 80 80 80s80-35.8 80-80c0-29.6-16.1-55.5-40-69.3l0 69.3c0 22.1-17.9 40-40 40s-40-17.9-40-40l0-69.3zm115.3-45.4c6.6 1.8 13.6 2.7 20.7 2.7c44.2 0 80-35.8 80-80s-35.8-80-80-80s-80 35.8-80 80c0 7.2 .9 14.1 2.7 20.7l49-49c15.6-15.6 40.9-15.6 56.6 0s15.6 40.9 0 56.6l-49 49zm-160-88.6c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L208 262.6l0 89.4c0 8.8 7.2 16 16 16s16-7.2 16-16l0-89.4 91.3-91.3c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L224 233.4l-84.7-84.7z";
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
    exports.faFluxCapacitor = exports.definition;
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

// src/_entries/faFluxCapacitor.js
var import_faFluxCapacitor = __toESM(require_faFluxCapacitor());
var export_faFluxCapacitor = import_faFluxCapacitor.definition;
export {
  export_faFluxCapacitor as faFluxCapacitor
};
