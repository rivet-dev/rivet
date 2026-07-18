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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFigma.js
var require_faFigma = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFigma.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "figma";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f799";
    var svgPathData = "M14 95.8C14 42.9 56.9 0 109.8 0L274.2 0c52.9 0 95.8 42.9 95.8 95.8 0 33.5-17.2 63-43.2 80.1 26 17.1 43.2 46.6 43.2 80.1 0 52.9-42.9 95.8-95.8 95.8l-2.1 0c-24.8 0-47.4-9.4-64.4-24.9l0 88.3c0 53.6-44 96.8-97.4 96.8-52.8 0-96.3-42.8-96.3-95.8 0-33.5 17.2-63 43.2-80.1-26-17.1-43.2-46.6-43.2-80.1s17.2-63 43.2-80.1C31.2 158.8 14 129.3 14 95.8zm162.3 95.8l-66.5 0c-35.6 0-64.4 28.8-64.4 64.4 0 35.4 28.6 64.2 64 64.4l66.9 0 0-128.8zM207.7 256c0 35.6 28.8 64.4 64.4 64.4l2.1 0c35.6 0 64.4-28.8 64.4-64.4s-28.8-64.4-64.4-64.4l-2.1 0c-35.6 0-64.4 28.8-64.4 64.4zm-97.9 95.8l-.4 0c-35.4 .2-64 29-64 64.4s29.2 64.4 64.9 64.4c36.3 0 66-29.4 66-65.5l0-63.4-66.5 0zm0-320.4c-35.6 0-64.4 28.8-64.4 64.4s28.8 64.4 64.4 64.4l66.5 0 0-128.8-66.5 0zm97.9 128.8l66.5 0c35.6 0 64.4-28.8 64.4-64.4s-28.8-64.4-64.4-64.4l-66.5 0 0 128.8z";
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
    exports.faFigma = exports.definition;
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

// src/_entries/faFigma.js
var import_faFigma = __toESM(require_faFigma());
var export_faFigma = import_faFigma.definition;
export {
  export_faFigma as faFigma
};
