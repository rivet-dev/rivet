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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHouseCircleExclamation.js
var require_faHouseCircleExclamation = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHouseCircleExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-circle-exclamation";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e50a";
    var svgPathData = "M496 544a144 144 0 1 1 0-288 144 144 0 1 1 0 288zM268.6 6.5c12.2-9.3 29.7-8.7 41.2 2l217.6 202c-10.2-1.7-20.7-2.6-31.3-2.6-78.6 0-146.1 47.2-175.8 114.8-5.1-1.8-10.5-2.8-16.2-2.8l-32 0c-26.5 0-48 21.5-48 48l0 96 91 0c6.1 17.3 14.6 33.4 25.1 48L144 512c-35.3 0-64-28.7-64-64l0-176-16 0c-13.2 0-25-8.1-29.8-20.3s-1.6-26.2 8-35.1l224-208 2.4-2zM496 444a20 20 0 1 0 0 40 20 20 0 1 0 0-40zm0-140c-8.8 0-16 7.2-16 16l0 80c0 8.8 7.2 16 16 16 8.8 0 16-7.2 16-16l0-80c0-8.8-7.2-16-16-16z";
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
    exports.faHouseCircleExclamation = exports.definition;
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

// src/_entries/faHouseCircleExclamation.js
var import_faHouseCircleExclamation = __toESM(require_faHouseCircleExclamation());
var export_faHouseCircleExclamation = import_faHouseCircleExclamation.definition;
export {
  export_faHouseCircleExclamation as faHouseCircleExclamation
};
