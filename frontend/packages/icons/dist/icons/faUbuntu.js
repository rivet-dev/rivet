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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUbuntu.js
var require_faUbuntu = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUbuntu.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ubuntu";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f7df";
    var svgPathData = "M469.2 75c0-20.1-8-39.3-22.2-53.5s-33.4-22.2-53.5-22.2-39.3 8-53.5 22.2-22.2 33.4-22.2 53.5c0 20.1 8 39.3 22.1 53.5s33.4 22.1 53.5 22.1 39.3-8 53.5-22.1 22.1-33.4 22.1-53.5l.1 0zm-315 165.7a75.6 75.6 0 1 0 -151.2 0 75.6 75.6 0 1 0 151.2 0zM57 346c18.6 46.9 51 87 93 115.1s91.5 42.6 142 41.7c-14.7-18.6-22.9-41.5-23.2-65.2-6.8-.9-13.3-2.1-19.5-3.4-26.8-5.7-51.9-17.3-73.6-34s-39.3-38.1-51.7-62.5c-20.9 9.9-44.5 12.8-67.1 8.2l.1 .1zm395.1 89.8a75.6 75.6 0 1 0 -151.2 0 75.6 75.6 0 1 0 151.2 0zM444 351.6c18.5 14.8 31.6 35.2 37.2 58.2 33.3-41.3 52.6-92.2 54.8-145.2s-12.5-105.4-42.2-149.4c-8.6 21.5-24 39.6-43.8 51.6 15.4 28.6 22.9 60.8 21.9 93.2s-10.7 64-28 91.6l.1 0zM101.1 135.4c12.4 2.7 24.3 7.5 35.1 14.3 16.6-24.2 38.9-44.1 64.8-58S255.8 70.4 285.2 70c.2-5.9 .9-11.9 2-17.7 3.6-16.7 11.1-32.3 21.8-45.5-47.7-3.8-95.4 6-137.6 28.5S94.3 91.7 70.8 133.4c2.7-.2 5.3-.3 8-.3 7.5 0 15 .8 22.4 2.3l-.1 0z";
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
    exports.faUbuntu = exports.definition;
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

// src/_entries/faUbuntu.js
var import_faUbuntu = __toESM(require_faUbuntu());
var export_faUbuntu = import_faUbuntu.definition;
export {
  export_faUbuntu as faUbuntu
};
