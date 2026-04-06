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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSchoolCircleExclamation.js
var require_faSchoolCircleExclamation = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSchoolCircleExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "school-circle-exclamation";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e56c";
    var svgPathData = "M64 128c-35.3 0-64 28.7-64 64L0 448c0 35.3 28.7 64 64 64l276 0c-10.5-14.6-19-30.7-25.1-48l-90.9 0 0-88c0-22.1 17.9-40 40-40l48 0c1 0 1.9 0 2.9 .1 26.3-74.6 97.5-128.1 181.1-128.1 28.6 0 55.6 6.2 80 17.4l0-33.4c0-35.3-28.7-64-64-64l-64 0-127.9-115.1c-18.3-16.4-46-16.4-64.2 0L128 128 64 128zm64 240c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32zM112 256l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16zM288 128a64 64 0 1 1 0 128 64 64 0 1 1 0-128zM496 544a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm0-100a20 20 0 1 1 0 40 20 20 0 1 1 0-40zm0-140c8.8 0 16 7.2 16 16l0 80c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-80c0-8.8 7.2-16 16-16z";
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
    exports.faSchoolCircleExclamation = exports.definition;
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

// src/_entries/faSchoolCircleExclamation.js
var import_faSchoolCircleExclamation = __toESM(require_faSchoolCircleExclamation());
var export_faSchoolCircleExclamation = import_faSchoolCircleExclamation.definition;
export {
  export_faSchoolCircleExclamation as faSchoolCircleExclamation
};
