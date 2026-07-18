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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBee.js
var require_faBee = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBee.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bee";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e0b2";
    var svgPathData = "M196.7 4.7c6.2-6.2 16.4-6.2 22.6 0l34.8 34.8C264.4 34.7 275.9 32 288 32s23.6 2.7 33.9 7.5L356.7 4.7c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6L347.8 58.8C360.4 73 368 91.6 368 112c0 3-.2 6-.5 8.9C390.3 105 419 96 448 96c65 0 128 44.9 128 112c0 61.9-53.6 104.9-112.9 111.2c.6-5.1 .9-10.2 .9-15.2c0-16.8-2.4-33.1-6.8-48.5c35.2-3.6 54.8-27.9 54.8-47.5c0-21.3-22.9-48-64-48c-34.8 0-56.6 19.2-62.4 38.1c28.6 26.3 46.4 64 46.4 105.9c0 83.8-103.6 179.7-129.9 202.8c-3.9 3.4-8.9 5.2-14.1 5.2s-10.2-1.8-14.1-5.2C247.6 483.7 144 387.8 144 304c0-41.9 17.9-79.6 46.4-105.9C184.6 179.2 162.8 160 128 160c-41.1 0-64 26.7-64 48c0 19.6 19.5 43.9 54.8 47.5c-4.4 15.4-6.8 31.7-6.8 48.5c0 5 .3 10.1 .9 15.2C53.6 312.9 0 269.9 0 208C0 140.9 63 96 128 96c29 0 57.7 9 80.5 24.9c-.3-2.9-.5-5.9-.5-8.9c0-20.4 7.6-39 20.2-53.2L196.7 27.3c-6.2-6.2-6.2-16.4 0-22.6zm52.8 384.1c12.8 16.4 26.5 31.3 38.5 43.4c12-12.1 25.7-27 38.5-43.4c1.2-1.6 2.5-3.2 3.7-4.8l-84.3 0c1.2 1.6 2.4 3.2 3.7 4.8zM359.6 336c5.6-12.6 8.4-23.4 8.4-32l-160 0c0 8.6 2.8 19.4 8.4 32l143.2 0zM352 256c-14.6-19.4-37.8-32-64-32s-49.4 12.6-64 32l128 0z";
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
    exports.faBee = exports.definition;
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

// src/_entries/faBee.js
var import_faBee = __toESM(require_faBee());
var export_faBee = import_faBee.definition;
export {
  export_faBee as faBee
};
