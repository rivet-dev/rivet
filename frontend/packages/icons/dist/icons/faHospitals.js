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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHospitals.js
var require_faHospitals = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHospitals.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hospitals";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f80e";
    var svgPathData = "M304 0c-26.5 0-48 21.5-48 48l0 16c35.3 0 64 28.7 64 64l0 352c0 11.7-3.1 22.6-8.6 32L528 512c26.5 0 48-21.5 48-48l0-80-112 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l112 0 0-64-112 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l112 0 0-208c0-26.5-21.5-48-48-48L304 0zM408 96c0-8.8 7.2-16 16-16l16 0c8.8 0 16 7.2 16 16l0 24 24 0c8.8 0 16 7.2 16 16l0 16c0 8.8-7.2 16-16 16l-24 0 0 24c0 8.8-7.2 16-16 16l-16 0c-8.8 0-16-7.2-16-16l0-24-24 0c-8.8 0-16-7.2-16-16l0-16c0-8.8 7.2-16 16-16l24 0 0-24zM0 144L0 384l144 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L0 416l0 48c0 26.5 21.5 48 48 48l192 0c26.5 0 48-21.5 48-48l0-320c0-26.5-21.5-48-48-48L48 96C21.5 96 0 117.5 0 144zm120 48c0-8.8 7.2-16 16-16l16 0c8.8 0 16 7.2 16 16l0 24 24 0c8.8 0 16 7.2 16 16l0 16c0 8.8-7.2 16-16 16l-24 0 0 24c0 8.8-7.2 16-16 16l-16 0c-8.8 0-16-7.2-16-16l0-24-24 0c-8.8 0-16-7.2-16-16l0-16c0-8.8 7.2-16 16-16l24 0 0-24z";
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
    exports.faHospitals = exports.definition;
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

// src/_entries/faHospitals.js
var import_faHospitals = __toESM(require_faHospitals());
var export_faHospitals = import_faHospitals.definition;
export {
  export_faHospitals as faHospitals
};
