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

// src/node_modules/@fortawesome/free-brands-svg-icons/faHips.js
var require_faHips = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faHips.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "hips";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f452";
    var svgPathData = "M251.6 157.6c0-1.9-.9-2.8-2.8-2.8l-40.9 0c-1.6 0-2.7 1.4-2.7 2.8l0 201.8c0 1.4 1.1 2.8 2.7 2.8l40.9 0c1.9 0 2.8-.9 2.8-2.8l0-201.8zM156.5 168c-16.1-11.8-36.3-17.9-60.3-18-18.1-.1-34.6 3.7-49.8 11.4l0-81.2c0-1.8-.9-2.7-2.8-2.7l-40.9 0C.9 77.5 0 78.4 0 80.2L0 359.4c0 1.9 .9 2.8 2.7 2.8l41 0c1.9 0 2.8-.9 2.8-2.8l0-136.1c0-.8-2.8-27 45.8-27 48.5 0 45.8 26.1 45.8 27l0 122.6c0 9 7.3 16.3 16.4 16.3l27.3 0c1.8 0 2.7-.9 2.7-2.8l0-136.1c0-23.4-9.3-41.8-28-55.3zM634.9 278.1c-6.8-15.7-18.4-27-34.9-34.1l-57.6-25.3c-8.6-3.6-9.2-11.2-2.6-16.1 7.4-5.5 44.3-13.9 84 6.8 1.7 1 4-.3 4-2.4l0-44.7c0-1.3-.6-2.1-1.9-2.6-17.7-6.6-36.1-9.9-55.1-9.9-26.5 0-45.3 5.8-58.5 15.4-.5 .4-28.4 20-22.7 53.7 3.4 19.6 15.8 34.2 37.2 43.6L580.4 286c11.6 5.1 15.2 13.3 12.2 21.2-3.7 9.1-13.2 13.6-36.5 13.6-24.3 0-44.7-8.9-58.4-19.1-2.1-1.4-4.4 .2-4.4 2.3l0 34.4c0 10.4 4.9 17.3 14.6 20.7 15.6 5.5 31.6 8.2 48.2 8.2 12.7 0 25.8-1.2 36.3-4.3 .7-.3 36-8.9 45.6-45.8 3.5-13.5 2.4-26.5-3.1-39.1zM376.2 149.8c-31.7 0-104.2 20.1-104.2 103.5l0 183.5c0 .8 .6 2.7 2.7 2.7l40.9 0c1.9 0 2.8-.9 2.8-2.7l0-88.8c16.5 12.7 35.8 19.1 57.7 19.1 60.5 0 108.7-48.5 108.7-108.7 .1-60.3-48.2-108.6-108.6-108.6zm0 46.6a62.2 62.2 0 1 1 0 124.3 62.2 62.2 0 1 1 0-124.3zM228.3 72.5c-15.9 0-28.8 12.9-28.9 28.9 0 15.6 12.7 28.9 28.9 28.9s28.9-13.1 28.9-28.9c0-16.2-13-28.9-28.9-28.9z";
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
    exports.faHips = exports.definition;
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

// src/_entries/faHips.js
var import_faHips = __toESM(require_faHips());
var export_faHips = import_faHips.definition;
export {
  export_faHips as faHips
};
