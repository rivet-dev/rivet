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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSchoolLock.js
var require_faSchoolLock = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSchoolLock.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "school-lock";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e56f";
    var svgPathData = "M0 192c0-35.3 28.7-64 64-64l64 0 127.9-115.1c18.3-16.4 46-16.4 64.2 0L448 128 512 128c33 0 60.2 25 63.6 57.2-14.7-5.9-30.8-9.2-47.6-9.2-70.7 0-128 57.3-128 128l0 24.4c-19.6 17.6-32 43.1-32 71.5l0 96c0 5.5 .5 10.9 1.3 16.1L64 512c-35.3 0-64-28.7-64-64L0 192zM332.3 341.5c-6-3.5-12.9-5.5-20.3-5.5l-48 0c-22.1 0-40 17.9-40 40l0 88 96 0 0-64.1c0-20.8 4.4-40.5 12.3-58.4zM112 384c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0zm16-144l0-32c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16zm224-48a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zM560 352l0-47.9c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 47.9 64 0zm-112 2.7l0-50.6c0-44.2 35.8-80 80-80s80 35.8 80 80l0 50.6c18.6 6.6 32 24.4 32 45.3l0 96c0 26.5-21.5 48-48 48l-128 0c-26.5 0-48-21.5-48-48l0-96c0-20.9 13.4-38.7 32-45.3z";
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
    exports.faSchoolLock = exports.definition;
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

// src/_entries/faSchoolLock.js
var import_faSchoolLock = __toESM(require_faSchoolLock());
var export_faSchoolLock = import_faSchoolLock.definition;
export {
  export_faSchoolLock as faSchoolLock
};
