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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCaduceus.js
var require_faCaduceus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCaduceus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "caduceus";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e681";
    var svgPathData = "M280 89.6c14.3-8.3 24-23.8 24-41.6c0-26.5-21.5-48-48-48s-48 21.5-48 48c0 17.8 9.7 33.3 24 41.6l0 22.4L171.5 71.7c-7.5-5-16.4-7.7-25.4-7.7c-11.6 0-22.9 4.4-31.3 12.4L8.8 175.7C3.2 181 0 188.4 0 196.1C0 211.5 12.5 224 27.9 224l110.8 0c33.5 0 66.2-10.5 93.3-30l0 136.5-20.6-12.3c-12-7.2-19.4-20.2-19.4-34.3l0-3.8c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 3.8c0 30.9 16.2 59.6 42.7 75.5l22.6 13.6-6.6 4c-26.5 15.9-42.7 44.5-42.7 75.5l0 3.8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-3.8c0-14.1 7.4-27.1 19.4-34.3l4.6-2.7 0 40.8 0 32c0 13.3 10.7 24 24 24s24-10.7 24-24l0-32 0-40.8 4.6 2.7c12 7.2 19.4 20.2 19.4 34.3l0 3.8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-3.8c0-30.9-16.2-59.6-42.7-75.5l-6.6-4 22.6-13.6c26.5-15.9 42.7-44.5 42.7-75.5l0-3.8c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 3.8c0 14.1-7.4 27.1-19.4 34.3L280 330.4 280 194c27.2 19.5 59.8 30 93.3 30l110.8 0c15.4 0 27.9-12.5 27.9-27.9c0-7.7-3.2-15.1-8.8-20.4l-106-99.3c-8.5-8-19.7-12.4-31.3-12.4c-9 0-17.9 2.7-25.4 7.7L280 112l0-22.4z";
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
    exports.faCaduceus = exports.definition;
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

// src/_entries/faCaduceus.js
var import_faCaduceus = __toESM(require_faCaduceus());
var export_faCaduceus = import_faCaduceus.definition;
export {
  export_faCaduceus as faCaduceus
};
