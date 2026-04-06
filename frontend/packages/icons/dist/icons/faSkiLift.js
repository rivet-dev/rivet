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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSkiLift.js
var require_faPersonSkiLift = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSkiLift.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-ski-lift";
    var width = 512;
    var height = 512;
    var aliases = ["ski-lift"];
    var unicode = "f7c8";
    var svgPathData = "M240 0c8.8 0 16 7.2 16 16l0 182.3-32 8.5L224 16c0-8.8 7.2-16 16-16zM64 80a48 48 0 1 1 96 0A48 48 0 1 1 64 80zm36.4 138c-14.2-30.8 8.3-66 42.2-66c18.2 0 34.8 10.7 42.3 27.3l28.8 63.4 55.9-14.9c25.4-6.8 50.3 12.4 50.3 38.6l0 122.1L443.1 343c12.6-4.6 20.9-16.6 20.9-30l0-1c0-13.3 10.7-24 24-24s24 10.7 24 24l0 1c0 33.5-20.9 63.4-52.3 75L128.3 510.5c-12.4 4.6-26.2-1.8-30.8-14.2s1.8-26.2 14.2-30.8L256 412.2l0-114.5-66.7 17.8c-22.6 6-46.2-5.1-55.9-26.3L100.4 218zM45.7 189.6l65 135.8c8.3 17.4 28 26.4 46.6 21.3l36.5-9.9c12.8-3.5 26 4.1 29.5 16.8s-4.1 26-16.8 29.5l-36.5 9.9c-41 11.2-84.2-8.6-102.5-46.9L2.3 210.4c-5.7-12-.7-26.3 11.3-32s26.3-.7 32 11.3z";
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
    exports.faPersonSkiLift = exports.definition;
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

// src/_entries/faSkiLift.js
var import_faPersonSkiLift = __toESM(require_faPersonSkiLift());
var export_faSkiLift = import_faPersonSkiLift.definition;
export {
  export_faSkiLift as faSkiLift
};
