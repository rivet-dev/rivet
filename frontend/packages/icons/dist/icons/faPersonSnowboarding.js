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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSnowboarding.js
var require_faPersonSnowboarding = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSnowboarding.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-snowboarding";
    var width = 576;
    var height = 512;
    var aliases = [127938, "snowboarding"];
    var unicode = "f7ce";
    var svgPathData = "M424.5 16a56 56 0 1 1 0 112 56 56 0 1 1 0-112zM166.4 45.5c10.2-14.4 30.2-17.9 44.6-7.7l272 192c14.4 10.2 17.9 30.2 7.7 44.6s-30.2 17.9-44.6 7.7l-92.2-65.1-62.2 53.3 32.1 26.7c18.2 15.2 28.8 37.7 28.8 61.5l0 87.8 77.5 15.2c6.2 1.2 12.6 .9 18.7-.8l41.2-11.8c12.7-3.6 26 3.7 29.7 16.5s-3.7 26-16.5 29.7l-41.2 11.8c-13.4 3.8-27.4 4.4-41.1 1.8L87.1 443.3c-17.2-3.4-33-11.8-45.3-24.1L15.5 393c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l26.2 26.2c5.6 5.6 12.8 9.4 20.6 11l64.2 12.6 0-123.7c0-27.7 12-54 32.8-72.2l69-60.4-88.2-62.3C159.6 80 156.2 60 166.4 45.5zm58.1 375.7l64 12.5 0-75.3c0-4.7-2.1-9.3-5.8-12.3l-58.2-48.5 0 123.6z";
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
    exports.faPersonSnowboarding = exports.definition;
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

// src/_entries/faPersonSnowboarding.js
var import_faPersonSnowboarding = __toESM(require_faPersonSnowboarding());
var export_faPersonSnowboarding = import_faPersonSnowboarding.definition;
export {
  export_faPersonSnowboarding as faPersonSnowboarding
};
