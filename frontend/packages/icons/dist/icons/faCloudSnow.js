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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCloudSnow.js
var require_faCloudSnow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCloudSnow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cloud-snow";
    var width = 512;
    var height = 512;
    var aliases = [127784];
    var unicode = "f742";
    var svgPathData = "M96 320c-53 0-96-43-96-96c0-42.5 27.6-78.6 65.9-91.2C64.7 126.1 64 119.1 64 112C64 50.1 114.1 0 176 0c43.1 0 80.5 24.3 99.2 60c14.7-17.1 36.5-28 60.8-28c44.2 0 80 35.8 80 80c0 5.5-.6 10.8-1.6 16c.5 0 1.1 0 1.6 0c53 0 96 43 96 96s-43 96-96 96L96 320zM64 352c13.3 0 24 10.7 24 24l0 16 16 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-16 0 0 16c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-16-16 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l16 0 0-16c0-13.3 10.7-24 24-24zm192 32c13.3 0 24 10.7 24 24l0 16 16 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-16 0 0 16c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-16-16 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l16 0 0-16c0-13.3 10.7-24 24-24zm216-8l0 16 16 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-16 0 0 16c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-16-16 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l16 0 0-16c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faCloudSnow = exports.definition;
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

// src/_entries/faCloudSnow.js
var import_faCloudSnow = __toESM(require_faCloudSnow());
var export_faCloudSnow = import_faCloudSnow.definition;
export {
  export_faCloudSnow as faCloudSnow
};
