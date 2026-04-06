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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceRelieved.js
var require_faFaceRelieved = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceRelieved.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-relieved";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e389";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM187.3 356.7c13 13 35.8 27.3 68.7 27.3s55.6-14.3 68.7-27.3c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6C329.4 397.2 298.9 416 256 416s-73.4-18.8-91.3-36.7c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0zM387.2 238.4c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2c17.6 23.5 52.8 23.5 70.4 0zm-262.4 0c17.6 23.5 52.8 23.5 70.4 0c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2zM207.2 74.9c2.8 8.4-1.7 17.4-10.1 20.2l-6.3 2.1c-23.6 7.9-44 23.4-57.8 44.1l-7.6 11.5c-4.9 7.4-14.8 9.3-22.2 4.4s-9.3-14.8-4.4-22.2l7.6-11.5c17.8-26.6 43.9-46.6 74.3-56.7l6.3-2.1c8.4-2.8 17.4 1.7 20.2 10.1zM314.9 95.2c-8.4-2.8-12.9-11.9-10.1-20.2s11.9-12.9 20.2-10.1l6.3 2.1c30.4 10.1 56.5 30.1 74.3 56.7l7.6 11.5c4.9 7.4 2.9 17.3-4.4 22.2s-17.3 2.9-22.2-4.4L379 141.4c-13.8-20.7-34.1-36.3-57.8-44.1l-6.3-2.1z";
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
    exports.faFaceRelieved = exports.definition;
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

// src/_entries/faFaceRelieved.js
var import_faFaceRelieved = __toESM(require_faFaceRelieved());
var export_faFaceRelieved = import_faFaceRelieved.definition;
export {
  export_faFaceRelieved as faFaceRelieved
};
