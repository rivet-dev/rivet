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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserDoctorMessage.js
var require_faUserDoctorMessage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserDoctorMessage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-doctor-message";
    var width = 640;
    var height = 512;
    var aliases = ["user-md-chat"];
    var unicode = "f82e";
    var svgPathData = "M96 128a128 128 0 1 1 256 0A128 128 0 1 1 96 128zM0 482.3c0-81 54-149.4 128-171.1l0 59.1c-23.1 6.9-40 28.3-40 53.7c0 30.9 25.1 56 56 56s56-25.1 56-56c0-25.4-16.9-46.8-40-53.7l0-65.4c6-.6 12.1-.9 18.3-.9l91.4 0c6.2 0 12.3 .3 18.3 .9l0 57.1c-27.6 7.1-48 32.2-48 62l0 40c0 8.8 7.2 16 16 16l16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l0-24c0-17.7 14.3-32 32-32s32 14.3 32 32l0 24c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0c8.8 0 16-7.2 16-16l0-40c0-29.8-20.4-54.9-48-62l0-50.8c74 21.7 128 90.1 128 171.1c0 16.4-13.3 29.7-29.7 29.7L29.7 512C13.3 512 0 498.7 0 482.3zM120 424a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zM432 0L592 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48l-48 0-83.2 62.4c-2.4 1.8-5.7 2.1-8.4 .8s-4.4-4.1-4.4-7.2l0-56-16 0c-26.5 0-48-21.5-48-48l0-128c0-26.5 21.5-48 48-48z";
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
    exports.faUserDoctorMessage = exports.definition;
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

// src/_entries/faUserMdChat.js
var import_faUserDoctorMessage = __toESM(require_faUserDoctorMessage());
var export_faUserMdChat = import_faUserDoctorMessage.definition;
export {
  export_faUserMdChat as faUserMdChat
};
